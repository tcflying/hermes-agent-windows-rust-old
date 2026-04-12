import subprocess
import os
import time
import tempfile
from pathlib import Path

EXE = r"G:\opencode-project\hermes-rs\target\debug\hermes-ui-bin.exe"
CDP_PORT = 9223

print("[1] Starting debug GUI with CDP...")
tmpdir = tempfile.mkdtemp(prefix="hermes-webview2-")
env = {
    **os.environ,
    "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS": f"--remote-debugging-port={CDP_PORT}",
    "WEBVIEW2_USER_DATA_FOLDER": tmpdir,
}
proc = subprocess.Popen([EXE], env=env)
print(f"  PID: {proc.pid}")

print("[2] Waiting for CDP port...")
cdp_ok = False
for i in range(30):
    time.sleep(1)
    try:
        import urllib.request

        resp = urllib.request.urlopen(
            f"http://localhost:{CDP_PORT}/json/version", timeout=2
        )
        print(f"  CDP ready! {resp.read()[:100]}")
        cdp_ok = True
        break
    except Exception as e:
        if proc.poll() is not None:
            print(f"  Process exited with code {proc.returncode}!")
            exit(1)
        print(f"  waiting... ({i + 1}) {e}")

if not cdp_ok:
    print("  CDP timeout - trying direct connection anyway...")

print("[3] Connecting Playwright...")
from playwright.sync_api import sync_playwright

with sync_playwright() as p:
    try:
        browser = p.chromium.connect_over_cdp(f"http://localhost:{CDP_PORT}")
        context = browser.contexts[0]
        page = context.pages[0]
        print(f"  Connected! URL: {page.url}, Title: {page.title()}")

        print("[4] Looking for textarea...")
        time.sleep(2)
        textarea = page.locator("textarea")
        count = textarea.count()
        print(f"  Found {count} textarea(s)")

        if count > 0:
            print("[5] Typing...")
            textarea.first.click()
            time.sleep(0.3)
            textarea.first.fill("Hello from Playwright!")
            time.sleep(0.5)

            print("[6] Sending...")
            send_btn = page.locator("button.send-btn")
            if send_btn.count() > 0:
                send_btn.first.click()
                print("  Send clicked!")
            else:
                textarea.first.press("Enter")
                print("  Enter pressed!")

            print("[7] Waiting for AI response...")
            time.sleep(20)

            messages = page.locator(".message")
            msg_count = messages.count()
            print(f"  Total messages: {msg_count}")
            for i in range(msg_count):
                msg = messages.nth(i)
                role = (
                    msg.locator(".message-role").text_content()
                    if msg.locator(".message-role").count() > 0
                    else "?"
                )
                content = (
                    msg.locator(".message-content").text_content()
                    if msg.locator(".message-content").count() > 0
                    else "(empty)"
                )
                print(f"  [{role}] {content[:300]}")

            status = page.locator(".status-dot")
            if status.count() > 0:
                cls = status.first.get_attribute("class") or ""
                print(f"  Backend: {'CONNECTED' if 'error' not in cls else 'OFFLINE'}")
        else:
            print("  No textarea - page content:")
            print(page.content()[:500])

    except Exception as e:
        print(f"  Playwright error: {e}")

print("\n[DONE]")
proc.terminate()
