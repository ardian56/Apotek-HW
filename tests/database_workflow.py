"""Run transaction checks against a temporary MariaDB: python3 tests/database_workflow.py."""
import os
from pathlib import Path
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory(prefix="apotek-test-db-") as directory:
    data = Path(directory)
    subprocess.run(["mariadb-install-db", "--no-defaults", f"--datadir={data}",
                    "--auth-root-authentication-method=normal", "--skip-test-db"],
                   stdout=subprocess.DEVNULL, check=True)
    server = subprocess.Popen(["mariadbd", "--no-defaults", f"--datadir={data}",
        f"--socket={data / 'mysql.sock'}", f"--pid-file={data / 'mysql.pid'}", f"--log-error={data / 'error.log'}",
        "--skip-networking", "--innodb-buffer-pool-size=32M", "--aria-pagecache-buffer-size=8M", "--key-buffer-size=8M"],
        stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
    try:
        for _ in range(150):
            if (data / "mysql.sock").exists():
                break
            if server.poll() is not None:
                raise RuntimeError((data / "error.log").read_text())
            time.sleep(0.1)
        else:
            raise RuntimeError("Temporary MariaDB did not start")
        env = os.environ.copy()
        env["APOTEK_TEST_SOCKET"] = str(data / "mysql.sock")
        subprocess.run(["cargo", "test", "--offline", "database_transaction_workflow", "--", "--ignored"],
                       cwd=ROOT, env=env, check=True)
    finally:
        server.terminate()
        server.wait(timeout=15)
