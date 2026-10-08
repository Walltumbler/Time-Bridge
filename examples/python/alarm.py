"""Local Python integration: standard library only. Run with Timebridge Desktop open."""
import json
import secrets
import time
import urllib.error
import urllib.request

ORIGIN = "https://example.com"
credential = None

def call(operation, **fields):
    headers = {"Content-Type": "application/json", "Origin": ORIGIN}
    if credential:
        headers["Authorization"] = "Bearer " + credential
    request = urllib.request.Request("http://127.0.0.1:47832/v1", json.dumps({"op": operation, **fields}).encode(), headers)
    # Never send local requests through a system proxy.
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    try:
        with opener.open(request, timeout=5) as response:
            return json.load(response)
    except urllib.error.HTTPError as error:
        raise RuntimeError(json.load(error).get("error", str(error))) from error

if __name__ == "__main__":
    call("detect")
    secret = secrets.token_hex(32)
    pairing = call("permissions.request", name="Example Python app", pollSecret=secret)
    print("Approve Example Python app in Timebridge Desktop, then wait here.")
    deadline = time.monotonic() + 120
    while time.monotonic() < deadline:
        state = call("permissions.poll", id=pairing["id"], pollSecret=secret)
        if state["status"] == "approved":
            credential = state["credential"]
            break
        if state["status"] == "denied":
            raise RuntimeError("Permission was declined")
        time.sleep(1.5)
    if not credential:
        raise RuntimeError("Approval expired")
    item = call("item.create", input={"kind": "countdown", "durationSeconds": 10})
    print("Created", item["title"], "— the timer continues after this script exits.")
