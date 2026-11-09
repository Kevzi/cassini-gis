"""Tests de la validation par llama.cpp sur le GGUF et des options d'accélération, sans torch
ni GPU : un faux `llama-server`, écrit par le test, répond comme le vrai.

    python3 -I -m unittest discover -s training/entrainement/tests
"""

import json
import stat
import sys
import tempfile
import textwrap
import types
import unittest
from pathlib import Path
from unittest import mock

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))

import pipeline  # noqa: E402
from data import Example  # noqa: E402

PLAN = '{"kind":"calls","calls":[{"command":"essai.synthetique","params":{"n":1}}]}'
QUESTION = '{"kind":"question","question":"Laquelle ?"}'

# Un faux serveur : il note chaque requête et rend PLAN pour un prompt qui contient « juste »,
# QUESTION sinon. Il n'écoute que l'adresse et le port qu'on lui donne.
FAKE_SERVER = textwrap.dedent(f"""\
    #!{sys.executable}
    import json, sys
    from http.server import BaseHTTPRequestHandler, HTTPServer
    args = sys.argv[1:]
    host, port = args[args.index("--host") + 1], int(args[args.index("--port") + 1])
    journal = open(sys.argv[0] + ".requetes", "a", encoding="utf-8")
    journal.write(json.dumps({{"argv": args}}) + "\\n"); journal.flush()
    class H(BaseHTTPRequestHandler):
        def log_message(self, *a): pass
        def reply(self, body):
            data = json.dumps(body).encode()
            self.send_response(200); self.send_header("Content-Length", str(len(data))); self.end_headers()
            self.wfile.write(data)
        def do_GET(self):
            self.reply({{"status": "ok"}})
        def do_POST(self):
            body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
            journal.write(json.dumps(body) + "\\n"); journal.flush()
            self.reply({{"content": {PLAN!r} if "juste" in body["prompt"] else {QUESTION!r}}})
    HTTPServer((host, port), H).serve_forever()
    """)


class GgufValidation(unittest.TestCase):
    def test_score_par_le_serveur_local_en_decodage_glouton(self):
        with tempfile.TemporaryDirectory() as folder:
            folder = Path(folder)
            server = folder / "llama-server"
            server.write_text(FAKE_SERVER, encoding="utf-8")
            server.chmod(server.stat().st_mode | stat.S_IXUSR)
            examples = [Example("prompt juste", PLAN + "<|im_end|>"),
                        Example("prompt faux", PLAN + "<|im_end|>"),
                        Example("prompt question", QUESTION + "<|im_end|>")]
            result = pipeline.validate_gguf(folder, ["-m", "modele.gguf"], "modele.gguf", examples,
                                            folder / "sorties.jsonl", folder / "serveur.log")
            self.assertEqual((result["justes"], result["total"]), (2, 3))
            self.assertEqual(result["modele"], "modele.gguf")
            rows = [json.loads(line) for line in (folder / "sorties.jsonl").read_text().splitlines()]
            self.assertEqual([r["juste"] for r in rows], [True, False, True])
            log = [json.loads(line) for line in (folder / "llama-server.requetes").read_text().splitlines()]
        argv = log[0]["argv"]
        self.assertEqual(argv[argv.index("--host") + 1], "127.0.0.1")
        self.assertEqual(argv[:2], ["-m", "modele.gguf"])
        requests = log[1:]
        self.assertEqual([r["prompt"] for r in requests], [e.prompt for e in examples])
        for r in requests:
            # Même décodage que la validation par PyTorch : glouton, même borne, arrêt à la fin de tour.
            self.assertEqual((r["temperature"], r["top_k"]), (0, 1))
            self.assertEqual(r["n_predict"], pipeline.HYPERPARAMETERS["jetons_max_generes"])
            self.assertEqual(r["stop"], ["<|im_end|>"])

    def test_un_serveur_qui_s_arrete_arrete_la_chaine(self):
        with tempfile.TemporaryDirectory() as folder:
            folder = Path(folder)
            server = folder / "llama-server"
            server.write_text("#!/bin/sh\nexit 3\n", encoding="utf-8")
            server.chmod(server.stat().st_mode | stat.S_IXUSR)
            with self.assertRaises(pipeline.PipelineStop):
                pipeline.validate_gguf(folder, [], "x", [Example("p", PLAN + "<|im_end|>")],
                                       folder / "sorties.jsonl", folder / "serveur.log")


class CudaToolkit(unittest.TestCase):
    def test_liens_idempotents_sous_le_dossier_des_roues(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder) / "cu13"
            (root / "bin").mkdir(parents=True)
            (root / "lib").mkdir()
            (root / "bin" / "nvcc").write_text("")
            for name in ("cudart", "cublas", "cublasLt"):
                (root / "lib" / f"lib{name}.so.13").write_text("")
            fake = types.ModuleType("nvidia")
            fake.__path__ = [folder]
            with mock.patch.dict(sys.modules, {"nvidia": fake}):
                self.assertEqual(pipeline.cuda_toolkit(), root)
                self.assertEqual(pipeline.cuda_toolkit(), root)
            self.assertEqual((root / "lib64").resolve(), (root / "lib").resolve())
            self.assertEqual((root / "lib" / "libcudart.so").resolve().name, "libcudart.so.13")

    def test_sans_nvcc_la_chaine_s_arrete_avec_un_message(self):
        with tempfile.TemporaryDirectory() as folder:
            (Path(folder) / "cu13").mkdir()
            fake = types.ModuleType("nvidia")
            fake.__path__ = [folder]
            with mock.patch.dict(sys.modules, {"nvidia": fake}), self.assertRaises(pipeline.PipelineStop):
                pipeline.cuda_toolkit()


if __name__ == "__main__":
    unittest.main()
