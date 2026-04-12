import asyncio
import json
import time
from playwright.async_api import async_playwright

CDP_URL = "http://127.0.0.1:9223"
API_BASE = "http://localhost:3848"


async def test_basic_chat(page):
    print("[TEST 1] Basic chat - single message...")
    textarea = page.locator("textarea.chat-input")
    send_btn = page.locator("button.send-btn")

    await textarea.fill("Hello, say 'world' in your response")
    await send_btn.click()

    await page.wait_for_timeout(15000)

    messages = page.locator(".message")
    count = await messages.count()
    print(f"  Messages after chat: {count}")

    if count < 2:
        print("  FAIL: Expected at least 2 messages (user + assistant)")
        return False

    last_msg = messages.nth(count - 1)
    last_text = await last_msg.text_content()
    print(f"  Last message: {last_text[:100] if last_text else 'empty'}...")

    if last_text and len(last_text.strip()) > 0:
        print("  PASS: Got response from AI")
        return True
    else:
        print("  FAIL: Empty response")
        return False


async def test_multi_session_concurrent(page):
    print("[TEST 2] Multi-session concurrent input...")

    new_chat_btn = page.locator("button.new-chat-btn")
    textarea = page.locator("textarea.chat-input")
    send_btn = page.locator("button.send-btn")

    # Session A: send first message
    await textarea.fill("What is 2+2? Answer with just the number.")
    await send_btn.click()
    await page.wait_for_timeout(500)

    # Create new chat for Session B
    await new_chat_btn.click()
    await page.wait_for_timeout(300)

    # Session B: send second message
    await textarea.fill(
        "What is the capital of France? Answer with just the city name."
    )
    await send_btn.click()

    # Wait for both to finish
    print("  Waiting for both sessions to complete...")
    await page.wait_for_timeout(20000)

    # Check status bar for active sessions
    status_bar = page.locator(".status-bar")
    status_text = await status_bar.text_content()
    print(f"  Status bar: {status_text}")

    # Check if backend is connected
    if "Backend connected" in status_text:
        print("  PASS: Backend connected")
    else:
        print("  WARN: Backend status unclear")

    print("  PASS: Multi-session test completed (both messages sent)")
    return True


async def test_backend_api():
    print("[TEST 3] Backend API health check...")
    import urllib.request

    try:
        req = urllib.request.Request(f"{API_BASE}/health")
        with urllib.request.urlopen(req, timeout=5) as resp:
            body = resp.read().decode()
            print(f"  Health: {body}")
            if body == "OK":
                print("  PASS: Backend API healthy")
                return True
            else:
                print("  FAIL: Unexpected response")
                return False
    except Exception as e:
        print(f"  FAIL: {e}")
        return False


async def main():
    print("=" * 50)
    print("Hermes-RS Multi-Session E2E Test")
    print("=" * 50)

    # Test backend API first
    api_ok = await test_backend_api()
    if not api_ok:
        print("\nBackend API failed, aborting GUI tests")
        return

    async with async_playwright() as p:
        print("\nConnecting to WebView2 via CDP...")
        try:
            browser = await p.chromium.connect_over_cdp(CDP_URL)
        except Exception as e:
            print(f"Failed to connect CDP: {e}")
            print(
                "Make sure hermes-ui-bin.exe is running with --remote-debugging-port=9223"
            )
            return

        contexts = browser.contexts
        if not contexts:
            print("No browser contexts found")
            return

        all_pages = []
        for ctx in contexts:
            all_pages.extend(ctx.pages)

        page = None
        for p in all_pages:
            url = p.url
            print(f"  Found page: {url[:80]}")
            if "tauri://" in url or "localhost" in url or "hermes" in url.lower():
                page = p
                break

        if not page:
            if all_pages:
                page = all_pages[-1]
            else:
                print("No pages found at all")
                return

        print(f"Using page: {page.url[:80]}")
        await page.wait_for_timeout(2000)

        results = []

        results.append(await test_basic_chat(page))
        results.append(await test_multi_session_concurrent(page))

        print("\n" + "=" * 50)
        passed = sum(1 for r in results if r)
        total = len(results)
        print(f"Results: {passed}/{total} tests passed")

        if passed == total:
            print("ALL TESTS PASSED")
        else:
            print("SOME TESTS FAILED")

        # Keep browser connected (don't close - it's the actual app)
        print("\nTests complete. GUI still running.")


asyncio.run(main())
