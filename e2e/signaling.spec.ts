import { test, expect } from "@playwright/test";
import { spawn, type ChildProcess } from "node:child_process";
import { createInterface } from "node:readline";
import path from "node:path";
import { execSync } from "node:child_process";

let signalServer: ChildProcess;
let roomUrl: string;

test.beforeAll(async () => {
  try {
    execSync("kill -9 $(lsof -ti :17777) 2>/dev/null || true", {
      timeout: 3000,
    });
  } catch {
    // ignore
  }

  signalServer = spawn(
    "cargo",
    ["run", "--package", "desktop", "--bin", "signal-server"],
    {
      cwd: path.resolve("src-tauri"),
      stdio: ["ignore", "pipe", "pipe"],
    },
  );

  signalServer.stderr!.on("data", (d) => {
    process.stderr.write("[signal-server] " + d.toString());
  });

  const rl = createInterface({ input: signalServer.stdout! });
  const urlLine = await new Promise<string>((resolve, reject) => {
    const timeout = setTimeout(
      () => reject(new Error("timeout waiting for ROOM_URL")),
      15000,
    );

    rl.on("line", (line) => {
      if (line.startsWith("ROOM_URL=")) {
        clearTimeout(timeout);
        resolve(line.slice("ROOM_URL=".length));
      }
    });
  });

  roomUrl = urlLine;
  console.log("signaling room URL:", roomUrl);
});

test.afterAll(() => {
  if (signalServer?.pid) {
    try {
      process.kill(signalServer.pid, "SIGKILL");
    } catch {
      // already dead
    }
  }
});

test("1 host + 3 guests join and connect via WebRTC", async ({ browser }) => {
  const hostContext = await browser.newContext();
  const guestContexts = [
    await browser.newContext(),
    await browser.newContext(),
    await browser.newContext(),
  ];

  const hostPage = await hostContext.newPage();

  // Host: open with ?role=host&room=<url>
  const hostUrl = `/?role=host&room=${encodeURIComponent(roomUrl)}`;
  await hostPage.goto(hostUrl);

  // Wait for host to connect to signaling
  await expect(
    hostPage
      .locator("text=Socket")
      .locator("..")
      .locator("span.text-slate-300"),
  ).toContainText("Connected", { timeout: 15000 });

  // Open 3 guest pages - all auto-join on mount
  const guestPages = await Promise.all(
    guestContexts.map(async (ctx) => {
      const page = await ctx.newPage();
      const guestUrl = `/?room=${encodeURIComponent(roomUrl)}`;
      await page.goto(guestUrl);
      return page;
    }),
  );

  // Wait for all guests to connect to signaling
  for (const guestPage of guestPages) {
    await expect(
      guestPage
        .locator("text=Socket")
        .locator("..")
        .locator("span.text-slate-300"),
    ).toContainText("Connected", { timeout: 15000 });
  }

  // Wait for host to see 3 peers (WebRTC connections established)
  await expect(
    hostPage
      .locator("text=Peers")
      .locator("..")
      .locator("span.text-slate-300"),
  ).toContainText("3", { timeout: 30000 });

  // Verify each guest sees "Host" label on their remote video (stream received)
  for (const guestPage of guestPages) {
    await expect(guestPage.getByText("Host")).toBeVisible({ timeout: 30000 });
  }

  // A guest camera must be renegotiated to every existing peer, including the host.
  await guestPages[0].getByTitle("Start camera").click();
  await expect
    .poll(
      () =>
        hostPage.locator("video").evaluateAll((videos) =>
          videos.slice(1).some((video) => video.srcObject instanceof MediaStream && video.srcObject.getVideoTracks().length > 0),
        ),
      { timeout: 30000 },
    )
    .toBe(true);

  // A host refresh creates a new peer ID and must trigger guests to renegotiate.
  await hostPage.reload();
  await expect(
    hostPage
      .locator("text=Peers")
      .locator("..")
      .locator("span.text-slate-300"),
  ).toContainText("3", { timeout: 30000 });
  for (const guestPage of guestPages) {
    await expect(
      guestPage
        .locator("text=Peers")
        .locator("..")
        .locator("span.text-slate-300"),
    ).toContainText("3", { timeout: 30000 });
  }

  // Cleanup
  await hostContext.close();
  for (const ctx of guestContexts) {
    await ctx.close();
  }
});

test("room chat broadcasts messages to connected participants", async ({ browser }) => {
  const hostContext = await browser.newContext();
  const guestContext = await browser.newContext();
  const hostPage = await hostContext.newPage();
  const guestPage = await guestContext.newPage();

  await hostPage.goto(`/?role=host&room=${encodeURIComponent(roomUrl)}`);
  await guestPage.goto(`/?room=${encodeURIComponent(roomUrl)}`);
  await expect(
    hostPage.locator("text=Peers").locator("..").locator("span.text-slate-300"),
  ).toContainText("1", { timeout: 30000 });

  await hostPage.getByLabel("Toggle chat").click();
  const hostInput = hostPage.getByLabel("Chat message");
  await hostInput.fill("Hello from the host");
  await hostInput.press("Enter");
  await expect(hostPage.getByText("Hello from the host")).toBeVisible();

  await expect(guestPage.getByLabel("Toggle chat")).toContainText("1");
  await guestPage.getByLabel("Toggle chat").click();
  await expect(guestPage.getByText("Hello from the host")).toBeVisible();

  const guestInput = guestPage.getByLabel("Chat message");
  await guestInput.fill("Hello from a guest");
  await guestInput.press("Enter");
  await expect(hostPage.getByText("Hello from a guest")).toBeVisible();

  await hostContext.close();
  await guestContext.close();
});
