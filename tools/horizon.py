#!/usr/bin/env python3
"""Export pinned sources, apply the soft_landing bundle and build a native Horizon."""
import argparse
import contextlib
import fcntl
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
COMPAT = json.loads((ROOT / "compatibility.json").read_text())
PIN = COMPAT["horizon_revision"]
MIXINS = Path(os.environ.get("HORIZON_MIXINS", ROOT.parent / "horizon-mixins")).resolve()

# The mod's staged suite: the gate module and its behavior tests compile into
# the landing file, which lives in the client lib package (landing, motion and
# their tests run under horizon-client-lib; the bin's own unittests are none).
# The two hooks are compiled and type-checked by `check`; the tests drive real
# falls over prepared 26.3 resources. The tests share the gate's atomics, so
# they run single-threaded.
SUITES = [
    (["-p", "horizon-client-lib", "--lib", "--no-default-features"],
     "landing::gate", ["--", "--test-threads", "1"]),
]

# The staged behavior tests load prepared resources; the engine's own landing
# and motion tests read the same env. First existing entry wins.
TEST_RESOURCES = [
    ROOT.parent / "horizon" / ".local" / "horizon-client" / "26.3",
    Path.home() / ".local" / "share" / "horizon" / ".horizon-cache" / "resourcepacks"
        / "7d8d6b710ab27c7f",
]


def test_env():
    env = dict(os.environ)
    if not env.get("HORIZON_TEST_RESOURCES"):
        for candidate in TEST_RESOURCES:
            if (candidate / "movement-properties.nbt").exists():
                env["HORIZON_TEST_RESOURCES"] = str(candidate)
                break
    return env


def digest(data):
    return hashlib.sha256(data).hexdigest()


def stage(horizon, stages):
    if not (horizon / ".git").exists():
        raise ValueError("--horizon must be a Git checkout (including a worktree)")
    subprocess.run(["git", "-C", str(horizon), "cat-file", "-e", PIN + "^{commit}"], check=True, capture_output=True)
    inputs = {"revision": PIN.encode(), "export-schema": b"2"}
    signature = digest(b"".join(k.encode() + b"\0" + value for k, value in sorted(inputs.items())))
    stages.mkdir(parents=True, exist_ok=True)
    out = stages / signature[:20]
    if out.exists():
        marker = json.loads((out / ".horizon-soft_landing.json").read_text())
        if marker["signature"] != signature:
            raise ValueError(f"stage marker mismatch: {out}")
        for path, expected in marker["files"].items():
            if digest((out / path).read_bytes()) != expected:
                raise ValueError(f"staged source was edited: {path}; remove this generated stage to rebuild it")
        return out, False
    temporary = Path(tempfile.mkdtemp(prefix=".soft-landing-stage-", dir=stages))
    try:
        process = subprocess.Popen(["git", "-C", str(horizon), "archive", "--format=tar", PIN], stdout=subprocess.PIPE)
        try:
            with tarfile.open(fileobj=process.stdout, mode="r|") as archive:
                for member in archive:
                    archive.extract(member, temporary, filter="data")
        finally:
            process.stdout.close()
            code = process.wait()
        if code:
            raise ValueError(f"git archive exited {code}")
        for path, expected in COMPAT["source_sha256"].items():
            if digest((temporary / path).read_bytes()) != expected:
                raise ValueError(f"pinned source hash mismatch: {path}")
        # Materialize the committed documentation symlink for the mixin CLI's
        # symlink-free source snapshot. No Rust source is edited here.
        for file in temporary.rglob("*"):
            if file.is_symlink():
                content = file.read_bytes()
                file.unlink()
                file.write_bytes(content)
        (temporary / ".horizon-revision").write_text(PIN + "\n")
        # The source tree itself is deterministic. Cargo outputs live elsewhere.
        files = {str(p.relative_to(temporary)): digest(p.read_bytes())
                 for p in sorted(temporary.rglob("*")) if p.is_file()}
        (temporary / ".horizon-soft_landing.json").write_text(json.dumps(
            {"signature": signature, "horizon_revision": PIN, "files": files}, sort_keys=True, indent=2) + "\n")
        os.rename(temporary, out)
    except BaseException:
        shutil.rmtree(temporary, ignore_errors=True)
        raise
    return out, True


def mixin_stage(horizon, stages):
    revision = subprocess.check_output(["git", "-C", str(MIXINS), "rev-parse", "HEAD"], text=True).strip()
    if revision != COMPAT["horizon_mixins_revision"]:
        raise ValueError(f"horizon-mixins must be pinned at {COMPAT['horizon_mixins_revision']}, got {revision}")
    subprocess.run(["git", "-C", str(MIXINS), "diff", "--quiet", "HEAD", "--", "Cargo.toml", "Cargo.lock", "crates"], check=True)
    original, _ = stage(horizon, stages / "original")
    subprocess.run(["cargo", "build", "--locked", "--offline", "--target-dir", str(MIXINS / "target"), "-p", "horizon-mixins-cli"], cwd=MIXINS, check=True, stdout=sys.stderr)
    cli = MIXINS / "target/debug/horizon-mixins"
    output = stages / "modded"
    plan = stages / "plan.json"
    base = ["--workspace", str(original), "--mods", str(ROOT / "mixins")]
    subprocess.run([str(cli), "plan", *base, "--out", str(plan)], check=True, capture_output=True, text=True)
    applied = subprocess.run([str(cli), "apply", *base, "--out", str(output)], check=True, capture_output=True, text=True)
    report = json.loads(applied.stdout)
    # The CLI drops a bundle whose targets no longer resolve and still says ok —
    # the hooks would then compile against nothing. The policy and gate modules
    # and the fall helper must be in the staged landing file, the capture helper
    # in the native tick hook's own file.
    landing = output / "crates/horizon-client/src/landing.rs"
    text = landing.read_text(errors="ignore")
    if "mod soft_landing {" not in text:
        raise ValueError("the staged landing.rs holds no `soft_landing` module: the bundle did not apply")
    if "pub mod gate {" not in text:
        raise ValueError("the staged landing.rs holds no `gate` module: the bundle did not apply")
    if "fn soft_landing_fall" not in text:
        raise ValueError("the staged landing.rs holds no `soft_landing_fall`: the bundle did not apply")
    hooks = output / "crates/horizon-client/src/game/native_hooks.rs"
    if "fn soft_landing_capture" not in hooks.read_text():
        raise ValueError("the staged native_hooks.rs holds no `soft_landing_capture`: the bundle did not apply")
    return output, report["stage_changed"]


@contextlib.contextmanager
def lock(path):
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a") as handle:
        fcntl.flock(handle, fcntl.LOCK_EX)
        yield


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["stage", "check", "build", "test"])
    parser.add_argument("--horizon", type=Path, default=ROOT.parent / "horizon")
    parser.add_argument("--target-dir", type=Path, default=ROOT / "target/horizon-cargo")
    args = parser.parse_args()
    horizon = args.horizon.resolve()
    target = args.target_dir.resolve()
    if target.is_relative_to(horizon):
        parser.error("Cargo target must be outside the original Horizon checkout")
    started = time.monotonic()
    with lock(ROOT / "target/stage.lock"):
        output, changed = mixin_stage(horizon, ROOT / "target/mixins")
        if target.is_relative_to(output) or output.is_relative_to(target):
            parser.error("Cargo target and generated source stage must be separate")
        report = {"stage": str(output), "stage_changed": changed,
                  "horizon_revision": PIN, "staging_seconds": time.monotonic()-started}
        summary = ROOT / "target" / f"{args.action}-run.log"
        if args.action != "stage":
            if args.action == "test":
                report["suites"] = []
                for package, suite_filter, tail in SUITES:
                    command = ["cargo", "test", "--locked", "--offline", *package,
                               suite_filter, "--target-dir", str(target), *tail]
                    run_start = time.monotonic()
                    result = subprocess.run(command, cwd=output, capture_output=True, text=True,
                                            env=test_env())
                    suite_report = {"filter": suite_filter, "package": package[1],
                                    "cargo_seconds": time.monotonic()-run_start,
                                    "cargo_command": command, "cargo_exit": result.returncode,
                                    "test_resources": test_env().get("HORIZON_TEST_RESOURCES")}
                    log = ROOT / "target" / f"test-cargo-{package[1].replace('::', '-')}.log"
                    log.write_text(result.stdout + result.stderr)
                    suite_report["cargo_log"] = str(log)
                    report["suites"].append(suite_report)
                    if result.returncode:
                        print(result.stderr[-12000:], file=sys.stderr)
                        print(json.dumps(report), file=sys.stderr)
                        summary.write_text(json.dumps(report, indent=2) + "\n")
                        return result.returncode
                report["cargo_exit"] = 0
            else:
                command = ["cargo", args.action, "--locked", "--offline", "-p", "horizon-client",
                           "--bin", "horizon-client", "--no-default-features", "--target-dir", str(target)]
                if args.action == "build":
                    command.extend(["--profile", "fast"])
                with lock(target.parent / (target.name + ".soft-landing.lock")):
                    run_start = time.monotonic()
                    result = subprocess.run(command, cwd=output, capture_output=True, text=True)
                report.update(cargo_seconds=time.monotonic()-run_start, cargo_command=command,
                              cargo_exit=result.returncode)
                log = ROOT / "target" / f"{args.action}-cargo.log"
                log.write_text(result.stdout + result.stderr)
                report["cargo_log"] = str(log)
                if result.returncode:
                    print(result.stderr[-12000:], file=sys.stderr)
                    print(json.dumps(report), file=sys.stderr)
                    summary.write_text(json.dumps(report, indent=2) + "\n")
                    return result.returncode
                if args.action == "build":
                    report["binary"] = str(target / "fast/horizon-client")
            summary.write_text(json.dumps(report, indent=2) + "\n")
        print(json.dumps(report, indent=2))
        return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        if isinstance(error, subprocess.CalledProcessError) and error.stderr:
            print(error.stderr, file=sys.stderr)
        print(f"Native mod: {error}", file=sys.stderr)
        raise SystemExit(1)
