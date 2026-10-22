"""Tests for the run service, against a temporary projects directory.

Run from `docker/python/` with plain Python 3.12 (no solver packages needed):

    python3 -m unittest discover -s tests -t .
"""

import json
import os
import sys
import tempfile
import textwrap
import threading
import tomllib
import unittest
import urllib.error
import urllib.request

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

import server  # noqa: E402

HERE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
LINUX = sys.platform.startswith("linux")


class ServiceCase(unittest.TestCase):
    """Starts the service on a free port over a scratch projects root."""

    config_overrides = {}

    @classmethod
    def setUpClass(cls):
        cls.root = tempfile.TemporaryDirectory()
        work = os.path.join(cls.root.name, "demo", "python", "work")
        os.makedirs(work)
        config = server.Config(
            {
                "PY_SERVICE_HOST": "127.0.0.1",
                "PY_SERVICE_PORT": "0",
                "SUPERMATH_PROJECTS": cls.root.name,
                "PY_MAX_TIMEOUT_S": "30",
                "PY_LOCK_PATH": os.path.join(HERE, "uv.lock"),
                "PY_SOLVERS_PATH": os.path.join(HERE, "solvers.toml"),
                **cls.config_overrides,
            }
        )
        cls.httpd = server.make_server(config)
        cls.base = f"http://127.0.0.1:{cls.httpd.server_port}"
        threading.Thread(target=cls.httpd.serve_forever, daemon=True).start()

    @classmethod
    def tearDownClass(cls):
        cls.httpd.shutdown()
        cls.httpd.server_close()
        cls.root.cleanup()

    def write(self, name, source):
        path = os.path.join(self.root.name, "demo", "python", "work", name)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w", encoding="utf-8") as handle:
            handle.write(textwrap.dedent(source))
        return "python/work/" + name

    def request(self, route, body=None):
        data = None if body is None else json.dumps(body).encode()
        req = urllib.request.Request(self.base + route, data=data, method="GET" if data is None else "POST")
        try:
            with urllib.request.urlopen(req, timeout=60) as response:
                return response.status, json.load(response)
        except urllib.error.HTTPError as error:
            return error.code, json.load(error)

    def run_file(self, name, source, entry, **extra):
        path = self.write(name, source)
        body = {"project": "demo", "path": path, "entry": entry, **extra}
        return self.request("/run", body)

    def search(self, name, source, predicate, frame, **extra):
        path = self.write(name, source)
        body = {"project": "demo", "path": path, "predicate": predicate, "frame": frame, **extra}
        return self.request("/search", body)


class RunTests(ServiceCase):
    def test_returns_the_value_with_provenance_fields(self):
        status, answer = self.run_file("add.py", "def add(a, b):\n    return {'sum': a + b}\n", "add", args={"a": 2, "b": 3})
        self.assertEqual(status, 200)
        self.assertEqual(answer["result"], "returned")
        self.assertEqual(answer["value"], {"sum": 5})
        self.assertEqual(len(answer["file_sha256"]), 64)
        self.assertEqual(len(answer["lock_sha256"]), 64)
        self.assertRegex(answer["python"], r"^3\.\d+\.\d+")
        self.assertGreaterEqual(answer["elapsed_ms"], 0)

    def test_integers_past_two_to_the_53_come_back_as_strings(self):
        _, answer = self.run_file("big.py", "def big():\n    return [2**64 + 1, 2**53, 7]\n", "big")
        self.assertEqual(answer["value"], [str(2**64 + 1), 2**53, 7])

    def test_captures_stdout_and_truncates_it(self):
        _, answer = self.run_file("talk.py", "def talk():\n    print('x' * 9000)\n    return 1\n", "talk")
        self.assertEqual(len(answer["stdout"]), 8000)
        self.assertTrue(answer["stdout_truncated"])

    def test_passes_seed_when_declared_and_seeds_random(self):
        source = "import random\ndef draw(seed):\n    return [seed, random.random()]\n"
        _, first = self.run_file("draw.py", source, "draw", seed=7)
        _, second = self.run_file("draw.py", source, "draw", seed=7)
        self.assertEqual(first["value"][0], 7)
        self.assertEqual(first["value"], second["value"])

    def test_hash_seed_is_zero(self):
        _, answer = self.run_file("env.py", "import os\ndef h():\n    return os.environ['PYTHONHASHSEED']\n", "h")
        self.assertEqual(answer["value"], "0")

    def test_an_exception_is_a_raised_result_with_a_traceback(self):
        source = "def inner():\n    raise ValueError('nope')\ndef outer():\n    inner()\n"
        status, answer = self.run_file("boom.py", source, "outer")
        self.assertEqual(status, 200)
        self.assertEqual(answer["result"], "raised")
        self.assertEqual(answer["exception"]["type"], "ValueError")
        self.assertEqual(answer["exception"]["message"], "nope")
        self.assertEqual(len(answer["exception"]["traceback"]), 2)

    def test_an_import_error_in_the_module_is_raised(self):
        _, answer = self.run_file("badimport.py", "import not_a_real_module\ndef f():\n    return 1\n", "f")
        self.assertEqual(answer["result"], "raised")
        self.assertEqual(answer["exception"]["type"], "ModuleNotFoundError")

    def test_a_value_that_is_not_json_is_raised(self):
        _, answer = self.run_file("setty.py", "def f():\n    return {1, 2}\n", "f")
        self.assertEqual(answer["result"], "raised")
        self.assertEqual(answer["exception"]["type"], "NotJsonSerializable")

    def test_sys_exit_is_a_raised_result(self):
        _, answer = self.run_file("quit.py", "import sys\ndef f():\n    sys.exit(3)\n", "f")
        self.assertEqual(answer["exception"]["type"], "SystemExit")

    def test_project_modules_import_each_other(self):
        self.write("helper.py", "def twice(n):\n    return 2 * n\n")
        _, answer = self.run_file("user.py", "from work.helper import twice\ndef f():\n    return twice(21)\n", "f")
        self.assertEqual(answer["value"], 42)

    def test_a_missing_entry_is_invalid_input(self):
        status, answer = self.run_file("noentry.py", "def f():\n    return 1\n", "g")
        self.assertEqual(status, 400)
        self.assertEqual(answer["error"]["code"], "invalid_input")

    def test_arguments_that_do_not_fit_are_invalid_input(self):
        status, answer = self.run_file("fit.py", "def f(a):\n    return a\n", "f", args={"b": 1})
        self.assertEqual(status, 400)
        self.assertIn("do not fit", answer["error"]["message"])

    def test_a_missing_file_is_not_found(self):
        body = {"project": "demo", "path": "python/work/absent.py", "entry": "f"}
        status, answer = self.request("/run", body)
        self.assertEqual(status, 404)
        self.assertEqual(answer["error"]["code"], "not_found")

    def test_paths_outside_the_package_are_refused(self):
        for path in ("../etc/passwd", "python/work/../x.py", "/abs.py", "lean/Supermath/A.lean"):
            status, _ = self.request("/run", {"project": "demo", "path": path, "entry": "f"})
            self.assertEqual(status, 400, path)

    def test_a_symlink_out_of_the_project_is_refused(self):
        outside = os.path.join(self.root.name, "outside.py")
        with open(outside, "w") as handle:
            handle.write("def f():\n    return 1\n")
        link = os.path.join(self.root.name, "demo", "python", "work", "escape.py")
        if not os.path.lexists(link):
            os.symlink(outside, link)
        status, _ = self.request("/run", {"project": "demo", "path": "python/work/escape.py", "entry": "f"})
        self.assertEqual(status, 400)

    def test_bad_requests_are_invalid_input(self):
        cases = [
            {"project": "Demo", "path": "python/work/a.py", "entry": "f"},
            {"project": "demo", "path": "python/work/a.py", "entry": "not a name"},
            {"project": "demo", "path": "python/work/a.py", "entry": "f", "args": [1]},
            {"project": "demo", "path": "python/work/a.py", "entry": "f", "seed": -1},
            {"project": "demo", "path": "python/work/a.py", "entry": "f", "timeout_s": 0},
            {"project": "demo", "path": "python/work/a.py", "entry": "f", "timeout_s": 31},
        ]
        for body in cases:
            status, _ = self.request("/run", body)
            self.assertEqual(status, 400, body)
        status, _ = self.request("/run", [1])
        self.assertEqual(status, 400)

    def test_unknown_routes_and_bodies(self):
        self.assertEqual(self.request("/nope")[0], 404)
        self.assertEqual(self.request("/nope", {})[0], 404)
        req = urllib.request.Request(self.base + "/run", data=b"not json", method="POST")
        with self.assertRaises(urllib.error.HTTPError) as caught:
            urllib.request.urlopen(req, timeout=10)
        self.assertEqual(caught.exception.code, 400)

    def test_a_run_past_its_timeout_is_killed_and_reported_as_a_limit(self):
        source = "import time\ndef spin():\n    time.sleep(60)\n"
        status, answer = self.run_file("slow.py", source, "spin", timeout_s=1)
        self.assertEqual(status, 200)
        self.assertEqual((answer["result"], answer["limit"]), ("limit", "time"))
        self.assertLess(answer["elapsed_ms"], 10_000)

    def test_a_busy_loop_is_killed_at_the_timeout(self):
        _, answer = self.run_file("busy.py", "def spin():\n    while True:\n        pass\n", "spin", timeout_s=1)
        self.assertEqual(answer["limit"], "time")

    def test_output_so_far_survives_a_kill(self):
        source = "import time\ndef f():\n    print('partial')\n    time.sleep(60)\n"
        _, answer = self.run_file("partial.py", source, "f", timeout_s=1)
        self.assertEqual(answer["limit"], "time")
        self.assertIn("partial", answer["stdout"])

    def test_ip_sockets_are_refused(self):
        source = textwrap.dedent(
            """
            import socket
            def f():
                s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
                try:
                    s.connect(('127.0.0.1', 9))
                except PermissionError:
                    return 'blocked'
                return 'connected or refused'
            """
        )
        _, answer = self.run_file("net.py", source, "f")
        self.assertEqual(answer["value"], "blocked")

    def test_the_working_directory_is_private_and_removed(self):
        source = "import os\ndef f():\n    open('scratch.txt', 'w').write('x')\n    return os.getcwd()\n"
        _, answer = self.run_file("tmp.py", source, "f")
        self.assertFalse(os.path.exists(answer["value"]))

    @unittest.skipUnless(LINUX, "RLIMIT_AS is only enforced on Linux")
    def test_a_memory_hog_is_reported_as_a_memory_limit(self):
        source = "def f():\n    return len(bytearray(8 * 1024**3))\n"
        _, answer = self.run_file("hog.py", source, "f")
        self.assertEqual((answer["result"], answer["limit"]), ("limit", "memory"))


class OutputLimitTests(ServiceCase):
    config_overrides = {"PY_OUTPUT_MB": "1"}

    def test_output_over_the_file_cap_is_a_limit(self):
        source = "def f():\n    for _ in range(2000):\n        print('y' * 1000)\n    return 1\n"
        _, answer = self.run_file("flood.py", source, "f")
        self.assertEqual((answer["result"], answer["limit"]), ("limit", "output"))


class SearchTests(ServiceCase):
    ODD = "def is_odd(n):\n    return n % 2 == 1\n"

    def test_finds_the_first_witness(self):
        status, answer = self.search("odd.py", self.ODD, "is_odd", {"kind": "range", "from": 1, "to": 100})
        self.assertEqual(status, 200)
        self.assertEqual(answer["result"], "found")
        self.assertEqual(answer["search"]["witness"], 2)
        self.assertEqual(answer["search"]["checked"], 1)
        self.assertEqual(answer["search"]["witness_json"], "2")

    def test_exhausting_the_frame_reports_how_much_was_checked(self):
        source = "def small(n):\n    return n < 1000\n"
        _, answer = self.search("small.py", source, "small", {"kind": "range", "from": 0, "to": 999})
        self.assertEqual(answer["result"], "exhausted")
        self.assertEqual(answer["search"]["checked"], 1000)
        self.assertEqual(answer["search"]["frame_size"], 1000)

    def test_a_product_frame_passes_tuples_in_lexicographic_order(self):
        source = "def below(t):\n    return not (t[0] == 1 and t[1] == 2)\n"
        frame = {"kind": "product", "ranges": [{"from": 0, "to": 2}, {"from": 0, "to": 3}]}
        _, answer = self.search("prod.py", source, "below", frame)
        self.assertEqual(answer["search"]["witness"], [1, 2])
        self.assertEqual(answer["search"]["checked"], 6)

    def test_a_generator_frame_is_counted(self):
        source = "def ok(n):\n    return n != 9\ndef values():\n    yield from range(5)\n"
        _, answer = self.search("gen.py", source, "ok", {"kind": "generator", "entry": "values"})
        self.assertEqual(answer["result"], "exhausted")
        self.assertEqual(answer["search"]["checked"], 5)

    def test_a_predicate_that_raises_stops_the_search(self):
        source = "def p(n):\n    return 1 // (n - 3) != 0\n"
        _, answer = self.search("raises.py", source, "p", {"kind": "range", "from": 0, "to": 10})
        self.assertEqual(answer["result"], "predicate_raised")
        self.assertEqual(answer["search"]["input"], 3)
        self.assertEqual(answer["search"]["exception"]["type"], "ZeroDivisionError")

    def test_a_predicate_that_returns_a_non_bool_stops_the_search(self):
        _, answer = self.search("nonbool.py", "def p(n):\n    return None\n", "p", {"kind": "range", "from": 0, "to": 3})
        self.assertEqual(answer["result"], "predicate_raised")
        self.assertEqual(answer["search"]["exception"]["type"], "NotBool")

    def test_the_admission_cap_stops_a_generator(self):
        source = "def p(n):\n    return True\ndef values():\n    yield from range(10**9)\n"
        _, answer = self.search("cap.py", source, "p", {"kind": "generator", "entry": "values"}, max_frame=500)
        self.assertEqual(answer["result"], "capped")
        self.assertEqual(answer["search"]["checked"], 500)

    def test_a_timeout_reports_the_progress_made(self):
        source = "import time\ndef p(n):\n    if n % 1024 == 1023:\n        time.sleep(0.3)\n    return True\n"
        frame = {"kind": "range", "from": 0, "to": 10**7}
        _, answer = self.search("long.py", source, "p", frame, timeout_s=2)
        self.assertEqual(answer["limit"], "time")
        self.assertGreater(answer["search"]["checked"], 0)

    def test_witnesses_larger_than_2_to_the_53_stay_exact_in_json(self):
        source = "def p(n):\n    return n != 2**60\ndef values():\n    yield 1\n    yield 2**60\n"
        _, answer = self.search("bigw.py", source, "p", {"kind": "generator", "entry": "values"})
        self.assertEqual(answer["search"]["witness"], str(2**60))
        self.assertEqual(answer["search"]["witness_json"], str(2**60))

    def test_bad_search_requests(self):
        path = self.write("odd2.py", self.ODD)
        for body in (
            {"project": "demo", "path": path, "predicate": "is_odd"},
            {"project": "demo", "path": path, "predicate": "is_odd", "frame": {"kind": "bogus"}},
            {"project": "demo", "path": path, "predicate": "nope", "frame": {"kind": "range", "from": 0, "to": 1}},
            {"project": "demo", "path": path, "predicate": "is_odd", "frame": {"kind": "range", "from": 0, "to": 1}, "max_frame": 0},
        ):
            self.assertEqual(self.request("/search", body)[0], 400, body)


class MetadataTests(ServiceCase):
    def test_health_reports_python_and_the_lock_digest(self):
        status, answer = self.request("/health")
        self.assertEqual(status, 200)
        self.assertEqual(answer["status"], "ok")
        self.assertEqual(answer["lock_sha256"], server.sha256_file(os.path.join(HERE, "uv.lock")))

    def test_solvers_are_the_lock_packages_with_lock_versions(self):
        _, answer = self.request("/solvers")
        versions = server.read_lock_versions(os.path.join(HERE, "uv.lock"))
        self.assertTrue(answer["solvers"])
        for row in answer["solvers"]:
            self.assertEqual(row["version"], versions[server.normalize(row["package"])])
            self.assertIn(row["area"], {"symbolic", "numeric", "sat", "smt", "optimization", "graphs"})
            self.assertTrue(row["summary"])

    def test_solvers_toml_matches_the_direct_dependencies_in_pyproject(self):
        with open(os.path.join(HERE, "pyproject.toml"), "rb") as handle:
            project = tomllib.load(handle)["project"]["dependencies"]
        declared = {server.normalize(d.split("=")[0].split(">")[0].split("<")[0].strip()) for d in project}
        rows = {server.normalize(r["package"]) for r in server.read_solvers(self.httpd_config())}
        self.assertEqual(rows, declared)

    def httpd_config(self):
        return server.Config(
            {
                "PY_LOCK_PATH": os.path.join(HERE, "uv.lock"),
                "PY_SOLVERS_PATH": os.path.join(HERE, "solvers.toml"),
            }
        )


class ConfigTests(unittest.TestCase):
    def test_defaults_and_bad_integers(self):
        config = server.Config({"PY_MEMORY_MB": "lots"})
        self.assertEqual(config.port, 3160)
        self.assertEqual(config.memory_mb, 4096)
        self.assertEqual(config.max_timeout_s, 900)


if __name__ == "__main__":
    unittest.main()
