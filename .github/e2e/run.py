"""End-to-end check of a client build: runs it against a stand-in PC Status server and
checks every status it sends.

    python .github/e2e/run.py --client path/to/pcsc-rs --hostname e2e

The server greets the client, asks for a few syncs, and fails the check when the client
exits early, sends something malformed, or (on macOS with a GPU in the I/O Registry)
reports no GPU. The runners have no real GPU, but the code that looks for one still
runs: DXGI and the performance counters on Windows, ioreg on macOS, sysfs on Linux.
"""

import argparse
import asyncio
import json
import os
import subprocess
import sys
import tempfile

import websockets

PASS = "e2e"


def check_status(status, hostname, errors, where):
    def fail(message):
        errors.append(f"{where}: {message}")

    def number(value, low=None, high=None):
        return isinstance(value, (int, float)) and not isinstance(value, bool) and (
            (low is None or value >= low) and (high is None or value <= high)
        )

    if not isinstance(status, dict):
        fail(f"status is not an object: {status!r}")
        return
    for key in ("_os", "version"):
        if not isinstance(status.get(key), str) or not status[key]:
            fail(f"{key} is missing or empty")
    if status.get("hostname") != hostname:
        fail(f"hostname is {status.get('hostname')!r}, not {hostname!r}")

    cpu = status.get("cpu") or {}
    if not isinstance(cpu.get("model"), str):
        fail("cpu.model is not a string")
    cores = cpu.get("cpus")
    if not isinstance(cores, list) or not cores:
        fail("cpu.cpus is empty")
    else:
        for i, core in enumerate(cores):
            if not number(core.get("cpu"), 0, 100):
                fail(f"core {i} usage is {core.get('cpu')!r}")
            if "model" in core and not isinstance(core["model"], str):
                fail(f"core {i} model is {core['model']!r}")
            if "max_mhz" in core and not number(core["max_mhz"], 1):
                fail(f"core {i} max_mhz is {core['max_mhz']!r}")

    for key in ("ram", "swap"):
        memory = status.get(key) or {}
        if not (number(memory.get("free"), 0) and number(memory.get("total"), 0)):
            fail(f"{key} is {memory!r}")
        elif memory["free"] > memory["total"]:
            fail(f"{key} has more free than total: {memory!r}")
    if not number((status.get("ram") or {}).get("total"), 1):
        fail("ram.total is 0")

    for i, storage in enumerate(status.get("storages") or []):
        if not isinstance(storage.get("name"), str):
            fail(f"storage {i} has no name")
        if not (number(storage.get("free"), 0) and number(storage.get("total"), 0)):
            fail(f"storage {i} is {storage!r}")

    if not number(status.get("uptime"), 1):
        fail(f"uptime is {status.get('uptime')!r}")
    loadavg = status.get("loadavg")
    if not (isinstance(loadavg, list) and len(loadavg) == 3 and all(number(v, 0) for v in loadavg)):
        fail(f"loadavg is {loadavg!r}")

    gpus = status.get("gpus")
    if not isinstance(gpus, list):
        fail(f"gpus is {gpus!r}")
        return
    for i, gpu in enumerate(gpus):
        if not isinstance(gpu.get("name"), str) or not gpu["name"]:
            fail(f"gpu {i} has no name")
        if not number(gpu.get("usage"), 0, 100):
            fail(f"gpu {i} usage is {gpu.get('usage')!r}")
        memory = gpu.get("memory") or {}
        if not (number(memory.get("free"), 0) and number(memory.get("total"), 0)):
            fail(f"gpu {i} memory is {memory!r}")
        elif memory["free"] > memory["total"]:
            fail(f"gpu {i} has more free memory than total: {memory!r}")
        if "temperature" in gpu and not number(gpu["temperature"]):
            fail(f"gpu {i} temperature is {gpu['temperature']!r}")


def gpu_expected():
    """macOS lists its GPUs as IOAccelerators; the client should report every one that
    publishes a usage. Elsewhere the runners may or may not show one."""
    if sys.platform != "darwin":
        return False
    out = subprocess.run(
        ["ioreg", "-r", "-d", "1", "-w", "0", "-c", "IOAccelerator"],
        capture_output=True,
        text=True,
        check=False,
    ).stdout
    print("ioreg IOAccelerator:\n" + (out or "(none)"))
    return "Device Utilization %" in out


async def run(args):
    errors = []
    statuses = []
    finished = asyncio.Event()

    async def handler(ws):
        await ws.send(json.dumps({"type": "Hi", "data": "hello"}))
        async for raw in ws:
            message = json.loads(raw)
            kind = message.get("type")
            if kind == "Hi":
                data = message.get("data") or {}
                if data.get("pass") != PASS:
                    errors.append("Hi does not carry the password")
                statuses.append(("Hi", data.get("data")))
            elif kind == "Sync":
                statuses.append((f"Sync {len(statuses)}", message.get("data")))
            else:
                errors.append(f"unexpected message: {raw[:200]}")
            if len(statuses) > args.syncs:
                finished.set()
                return
            # The client sends its status each time the server asks
            await asyncio.sleep(1)
            await ws.send(json.dumps({"type": "Sync", "data": "sync"}))

    async with websockets.serve(handler, "127.0.0.1", 0) as server:
        port = server.sockets[0].getsockname()[1]
        env = dict(
            os.environ,
            PASS=PASS,
            PCSC_URI=f"ws://127.0.0.1:{port}",
            HOSTNAME=args.hostname,
        )
        # An empty working directory: no .env from the checkout
        with tempfile.TemporaryDirectory() as cwd:
            client = subprocess.Popen(
                [os.path.abspath(args.client)],
                env=env,
                cwd=cwd,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                text=True,
                errors="replace",
            )
            try:
                await asyncio.wait_for(finished.wait(), args.timeout)
            except TimeoutError:
                errors.append(f"only {len(statuses)} statuses in {args.timeout} s")
            exited = client.poll()
            if exited is not None:
                errors.append(f"the client exited with {exited}")
            client.terminate()
            try:
                output, _ = client.communicate(timeout=10)
            except subprocess.TimeoutExpired:
                client.kill()
                output, _ = client.communicate()

    print("--- client output ---\n" + output.strip())
    for where, status in statuses:
        check_status(status, args.hostname, errors, where)
    if statuses:
        last = statuses[-1][1] or {}
        print("--- last status ---")
        cpu = last.get("cpu") or {}
        summary = {
            "_os": last.get("_os"),
            "version": last.get("version"),
            "cpu": cpu.get("model"),
            "cores": len(cpu.get("cpus") or []),
            "max_mhz": sorted({c["max_mhz"] for c in cpu.get("cpus") or [] if "max_mhz" in c}),
            "gpus": last.get("gpus"),
        }
        print(json.dumps(summary, indent=1, ensure_ascii=False))
        if gpu_expected() and not last.get("gpus"):
            errors.append("the I/O Registry has a GPU, but the client reported none")
    if errors:
        print("--- failed ---")
        for error in errors:
            print(error)
        return 1
    print(f"--- ok: {len(statuses)} statuses ---")
    return 0


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--client", required=True)
    parser.add_argument("--hostname", default="e2e")
    parser.add_argument("--syncs", type=int, default=3)
    parser.add_argument("--timeout", type=int, default=120)
    sys.exit(asyncio.run(run(parser.parse_args())))


if __name__ == "__main__":
    main()
