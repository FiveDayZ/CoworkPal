import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdtempSync, readFileSync, readdirSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

const token = "test-token-that-is-longer-than-thirty-two-characters";
const adminToken = "test-admin-token-that-is-longer-than-thirty-two-characters";
const waitingToken = "waiting-token-that-is-longer-than-thirty-two-characters";

test("stores and returns an authenticated snapshot", async (context) => {
  const dataDir = mkdtempSync(join(tmpdir(), "coworkpal-sync-"));
  const port = 18000 + Math.floor(Math.random() * 1000);
  const child = spawn(process.execPath, ["server.mjs"], {
    cwd: import.meta.dirname,
    env: {
      ...process.env,
      PORT: String(port),
      HOST: "127.0.0.1",
      DATA_DIR: dataDir,
      COWORKPAL_USERS_JSON: JSON.stringify({ tester: token, waiting: waitingToken }),
      COWORKPAL_ADMIN_TOKEN: adminToken,
    },
    stdio: "ignore",
  });
  context.after(() => child.kill());

  const base = `http://127.0.0.1:${port}`;
  await waitForHealth(base);

  const dashboard = await fetch(base);
  assert.equal(dashboard.status, 200);
  assert.match(dashboard.headers.get("content-type"), /^text\/html/);
  assert.match(dashboard.headers.get("content-security-policy"), /default-src 'self'/);
  const dashboardHtml = await dashboard.text();
  assert.match(dashboardHtml, /CoworkPal 云端总览/);
  assert.match(dashboardHtml, /日报 \/ 体检/);
  assert.match(dashboardHtml, /已解锁成就/);

  for (const asset of ["dashboard.css", "dashboard.js", "cocat-avatar.png"]) {
    const response = await fetch(`${base}/${asset}`);
    assert.equal(response.status, 200, `${asset} should be served`);
    assert.ok((await response.arrayBuffer()).byteLength > 0);
  }
  const dashboardScript = await (await fetch(`${base}/dashboard.js`)).text();
  assert.match(dashboardScript, /detailGroup\("云端数据"/);
  assert.match(dashboardScript, /\["日报",/);
  assert.match(dashboardScript, /\["体检",/);
  assert.match(dashboardScript, /\["成就",/);

  const unauthorized = await fetch(`${base}/v1/snapshot`);
  assert.equal(unauthorized.status, 401);

  const unauthorizedAdmin = await fetch(`${base}/v1/admin/snapshots`);
  assert.equal(unauthorizedAdmin.status, 401);

  const invalidTokenRequest = await fetch(`${base}/v1/token-requests`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ userName: "" }),
  });
  assert.equal(invalidTokenRequest.status, 400);

  const tokenRequestResponse = await fetch(`${base}/v1/token-requests`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ userName: "new-user", deviceId: "cocat-new" }),
  });
  assert.equal(tokenRequestResponse.status, 201);
  const tokenRequest = await tokenRequestResponse.json();
  assert.equal(tokenRequest.status, "pending");
  assert.ok(tokenRequest.requestId);
  assert.ok(tokenRequest.claimSecret.length >= 32);

  const unauthorizedTokenStatus = await fetch(
    `${base}/v1/token-requests/${tokenRequest.requestId}`,
  );
  assert.equal(unauthorizedTokenStatus.status, 401);
  const pendingTokenStatus = await fetch(`${base}/v1/token-requests/${tokenRequest.requestId}`, {
    headers: { Authorization: `Bearer ${tokenRequest.claimSecret}` },
  });
  assert.equal(pendingTokenStatus.status, 200);
  assert.equal((await pendingTokenStatus.json()).status, "pending");

  const invalidUpload = await fetch(`${base}/v1/snapshot`, {
    method: "PUT",
    headers: {
      Authorization: `Bearer ${token}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ schemaVersion: 1 }),
  });
  assert.equal(invalidUpload.status, 400);

  const snapshot = {
    schemaVersion: 1,
    exportedAt: Date.now(),
    appVersion: "0.3.4-test",
    deviceProfile: {
      capturedAt: Date.now(),
      cpuName: "Test CPU 9000",
      gpuName: "Test GPU 8000",
      cpuPhysicalCoreCount: 8,
      cpuLogicalCoreCount: 16,
      totalMemoryBytes: 32 * 1024 ** 3,
      gpuMemoryTotalBytes: 12 * 1024 ** 3,
      inventory: {
        motherboard: [{ name: "Test Board", vendor: "CoworkPal Labs" }],
        memoryModules: [
          {
            manufacturer: "Test Memory",
            partNumber: "TM-16G",
            capacityBytes: 16 * 1024 ** 3,
            speedMhz: 6000,
          },
        ],
        gpus: [{ name: "Test GPU 8000", capacityBytes: 12 * 1024 ** 3 }],
        displays: [],
        disks: [{ name: "Test NVMe", capacityBytes: 1024 ** 4 }],
        audioDevices: [],
        networkAdapters: [{ name: "Test Ethernet" }],
      },
    },
    settings: {
      schemaVersion: 1,
      catId: "cocat-01",
      themeName: "coworkpal",
      launchAtStartup: true,
      enableLowPowerMode: false,
      enableStaticCatMode: false,
      enableSleepMode: true,
      enableNotifications: true,
      integratedHardwareMonitorEnabled: true,
      samplingIntervalMs: 1_000,
      backgroundSamplingIntervalMs: 5_000,
      monitorBarMode: "Expanded",
      visibleMonitorMetrics: ["Cpu", "Ram", "Gpu"],
    },
    workshop: {
      schemaVersion: 1,
      workshopLevel: 8,
      catAffinityLevel: 5,
      totalOnlineSeconds: 90_061,
      parts: 321,
      insight: 45,
      todayParts: 12,
      todayInsight: 4,
      moduleLevels: { cpu: { parts: 4, process: 3 } },
    },
    layout: { schemaVersion: 1 },
    workLogs: { schemaVersion: 1, entries: { "2026-08-25": {} } },
    focusSessions: { schemaVersion: 1, sessions: [{ id: "focus-1" }] },
    achievements: { schemaVersion: 1, unlocks: { first_backup: {} } },
    notes: { schemaVersion: 1, notes: [{ id: "note-1" }, { id: "note-2" }] },
  };
  const upload = await fetch(`${base}/v1/snapshot`, {
    method: "PUT",
    headers: {
      Authorization: `Bearer ${token}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify(snapshot),
  });
  assert.equal(upload.status, 201);

  const download = await fetch(`${base}/v1/snapshot`, {
    headers: { Authorization: `Bearer ${token}` },
  });
  assert.equal(download.status, 200);
  const envelope = await download.json();
  assert.deepEqual(envelope.data, snapshot);
  assert.ok(envelope.revision);
  assert.ok(readFileSync(findLatest(dataDir), "utf8").includes(envelope.revision));

  const adminResponse = await fetch(`${base}/v1/admin/snapshots`, {
    headers: { Authorization: `Bearer ${adminToken}` },
  });
  assert.equal(adminResponse.status, 200);
  const overview = await adminResponse.json();
  assert.equal(overview.users.length, 2);
  assert.equal(overview.tokenRequests.length, 1);
  assert.equal(overview.tokenRequests[0].status, "pending");
  assert.equal("claimSecret" in overview.tokenRequests[0], false);
  assert.equal("accessToken" in overview.tokenRequests[0], false);
  const tester = overview.users.find((user) => user.userName === "tester");
  const waiting = overview.users.find((user) => user.userName === "waiting");
  assert.equal(waiting.backup, null);
  assert.equal(tester.backup.status, "ready");
  assert.equal(tester.backup.appVersion, "0.3.4-test");
  assert.equal(tester.backup.device.catId, "cocat-01");
  assert.equal(tester.backup.device.monitorBarMode, "Expanded");
  assert.equal(tester.backup.hardware.cpuName, "Test CPU 9000");
  assert.equal(tester.backup.hardware.gpuName, "Test GPU 8000");
  assert.equal(tester.backup.hardware.cpuLogicalCoreCount, 16);
  assert.equal(tester.backup.hardware.totalMemoryBytes, 32 * 1024 ** 3);
  assert.equal(tester.backup.hardware.motherboard[0].name, "Test Board");
  assert.equal(tester.backup.hardware.memoryModules[0].speedMhz, 6000);
  assert.equal(tester.backup.hardware.disks[0].name, "Test NVMe");
  assert.equal(tester.backup.workshop.level, 8);
  assert.equal(tester.backup.workshop.totalOnlineSeconds, 90_061);
  assert.deepEqual(tester.backup.workshop.moduleLevels.cpu, { parts: 4, process: 3 });
  assert.deepEqual(tester.backup.counts, {
    notes: 2,
    focusSessions: 1,
    workLogDays: 1,
    achievements: 1,
  });

  const approve = await fetch(
    `${base}/v1/admin/token-requests/${tokenRequest.requestId}/approve`,
    {
      method: "POST",
      headers: { Authorization: `Bearer ${adminToken}` },
    },
  );
  assert.equal(approve.status, 200);
  assert.equal((await approve.json()).status, "approved");

  const duplicateApprove = await fetch(
    `${base}/v1/admin/token-requests/${tokenRequest.requestId}/approve`,
    {
      method: "POST",
      headers: { Authorization: `Bearer ${adminToken}` },
    },
  );
  assert.equal(duplicateApprove.status, 409);

  const approvedTokenStatus = await fetch(`${base}/v1/token-requests/${tokenRequest.requestId}`, {
    headers: { Authorization: `Bearer ${tokenRequest.claimSecret}` },
  });
  assert.equal(approvedTokenStatus.status, 200);
  const approvedRequest = await approvedTokenStatus.json();
  assert.equal(approvedRequest.status, "approved");
  assert.ok(approvedRequest.accessToken.length >= 32);

  const firstAutomaticBackup = await fetch(`${base}/v1/snapshot`, {
    method: "PUT",
    headers: {
      Authorization: `Bearer ${approvedRequest.accessToken}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify(snapshot),
  });
  assert.equal(firstAutomaticBackup.status, 201);

  const claimedTokenStatus = await fetch(`${base}/v1/token-requests/${tokenRequest.requestId}`, {
    headers: { Authorization: `Bearer ${tokenRequest.claimSecret}` },
  });
  const claimedRequest = await claimedTokenStatus.json();
  assert.equal(claimedRequest.status, "approved");
  assert.equal("accessToken" in claimedRequest, false);

  const finalAdminResponse = await fetch(`${base}/v1/admin/snapshots`, {
    headers: { Authorization: `Bearer ${adminToken}` },
  });
  const finalOverview = await finalAdminResponse.json();
  assert.equal(finalOverview.users.length, 3);
  assert.equal(finalOverview.users.find((user) => user.userName === "new-user").backup.status, "ready");
  const authState = readFileSync(join(dataDir, "auth-state.json"), "utf8");
  assert.equal(authState.includes(approvedRequest.accessToken), false);

  const rejectedResponse = await fetch(`${base}/v1/token-requests`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ userName: "rejected-user", deviceId: "cocat-rejected" }),
  });
  const rejectedRequest = await rejectedResponse.json();
  const reject = await fetch(
    `${base}/v1/admin/token-requests/${rejectedRequest.requestId}/reject`,
    {
      method: "POST",
      headers: { Authorization: `Bearer ${adminToken}` },
    },
  );
  assert.equal(reject.status, 200);
  const rejectedStatus = await fetch(`${base}/v1/token-requests/${rejectedRequest.requestId}`, {
    headers: { Authorization: `Bearer ${rejectedRequest.claimSecret}` },
  });
  const rejected = await rejectedStatus.json();
  assert.equal(rejected.status, "rejected");
  assert.equal("accessToken" in rejected, false);
});

async function waitForHealth(base) {
  for (let attempt = 0; attempt < 50; attempt += 1) {
    try {
      if ((await fetch(`${base}/health`)).ok) return;
    } catch {}
    await new Promise((resolve) => setTimeout(resolve, 50));
  }
  throw new Error("server did not start");
}

function findLatest(dataDir) {
  const userDir = join(dataDir, "users", readdirSync(join(dataDir, "users"))[0]);
  return join(userDir, "latest.json");
}
