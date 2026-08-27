import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdtempSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
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
  assert.match(dashboardScript, /backupHistoryGroup/);
  assert.match(dashboardScript, /推送还原/);
  assert.match(dashboardScript, /删除用户/);

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
  assert.equal(tester.tokens[0].accessToken, token);
  assert.equal(tester.tokens[0].source, "configured");
  const configuredDelete = await fetch(`${base}/v1/admin/users/${tester.userId}`, {
    method: "DELETE",
    headers: { Authorization: `Bearer ${adminToken}` },
  });
  assert.equal(configuredDelete.status, 409);
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
  assert.match(approvedRequest.userId, /^[0-9a-f-]{36}$/);

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
  const newUser = finalOverview.users.find((user) => user.userName === "new-user");
  assert.equal(newUser.backup.status, "ready");
  assert.equal(newUser.userId, approvedRequest.userId);
  assert.equal(newUser.tokens[0].accessToken, approvedRequest.accessToken);
  assert.equal(newUser.tokens[0].status, "active");
  assert.equal(newUser.historyCount, 1);
  const authState = readFileSync(join(dataDir, "auth-state.json"), "utf8");
  assert.equal(authState.includes(approvedRequest.accessToken), false);

  const historyResponse = await fetch(`${base}/v1/snapshot/history`, {
    headers: { Authorization: `Bearer ${approvedRequest.accessToken}` },
  });
  assert.equal(historyResponse.status, 200);
  const history = await historyResponse.json();
  assert.equal(history.userId, approvedRequest.userId);
  assert.equal(history.versions.length, 1);

  const recoveryResponse = await fetch(`${base}/v1/token-recoveries`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ userId: approvedRequest.userId, deviceId: "replacement-device" }),
  });
  assert.equal(recoveryResponse.status, 201);
  const recoveryRequest = await recoveryResponse.json();
  assert.equal(recoveryRequest.kind, "recovery");
  assert.equal(recoveryRequest.userId, approvedRequest.userId);

  const approveRecovery = await fetch(
    `${base}/v1/admin/token-requests/${recoveryRequest.requestId}/approve`,
    { method: "POST", headers: { Authorization: `Bearer ${adminToken}` } },
  );
  assert.equal(approveRecovery.status, 200);
  const recoveryStatusResponse = await fetch(
    `${base}/v1/token-requests/${recoveryRequest.requestId}`,
    { headers: { Authorization: `Bearer ${recoveryRequest.claimSecret}` } },
  );
  const recoveryStatus = await recoveryStatusResponse.json();
  assert.equal(recoveryStatus.accessToken, approvedRequest.accessToken);
  const restoredSnapshot = await fetch(`${base}/v1/snapshot`, {
    headers: { Authorization: `Bearer ${recoveryStatus.accessToken}` },
  });
  assert.equal(restoredSnapshot.status, 200);
  assert.deepEqual((await restoredSnapshot.json()).data, snapshot);

  const tokenId = newUser.tokens[0].tokenId;
  const rotateResponse = await fetch(
    `${base}/v1/admin/users/${newUser.userId}/tokens/${tokenId}/rotate`,
    { method: "POST", headers: { Authorization: `Bearer ${adminToken}` } },
  );
  assert.equal(rotateResponse.status, 200);
  const rotated = await rotateResponse.json();
  assert.ok(rotated.accessToken.length >= 32);
  assert.notEqual(rotated.accessToken, approvedRequest.accessToken);
  assert.equal(
    (await fetch(`${base}/v1/snapshot`, {
      headers: { Authorization: `Bearer ${approvedRequest.accessToken}` },
    })).status,
    401,
  );
  assert.equal(
    (await fetch(`${base}/v1/snapshot`, {
      headers: { Authorization: `Bearer ${rotated.accessToken}` },
    })).status,
    200,
  );
  const rotatedUpload = await fetch(`${base}/v1/snapshot`, {
    method: "PUT",
    headers: {
      Authorization: `Bearer ${rotated.accessToken}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ ...snapshot, appVersion: "0.3.4-rotated-token" }),
  });
  assert.equal(rotatedUpload.status, 201);
  assert.equal((await adminOverview(base)).users.length, 3);

  const revokeResponse = await fetch(
    `${base}/v1/admin/users/${newUser.userId}/tokens/${tokenId}/revoke`,
    { method: "POST", headers: { Authorization: `Bearer ${adminToken}` } },
  );
  assert.equal(revokeResponse.status, 200);
  assert.equal(
    (await fetch(`${base}/v1/snapshot`, {
      headers: { Authorization: `Bearer ${rotated.accessToken}` },
    })).status,
    401,
  );

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

test("preserves a legacy issued token after the client authenticates", async (context) => {
  const dataDir = mkdtempSync(join(tmpdir(), "coworkpal-sync-legacy-token-"));
  const port = 19000 + Math.floor(Math.random() * 1000);
  const legacyToken = "legacy-issued-token-that-is-longer-than-thirty-two-characters";
  const legacyUserId = createHash("sha256").update("legacy-user").digest("hex");
  const requestId = "c3658617-2597-4bd3-a554-3bde37d53d21";
  writeFileSync(
    join(dataDir, "auth-state.json"),
    JSON.stringify({
      schemaVersion: 1,
      issuedUsers: [
        {
          id: legacyUserId,
          name: "legacy-user",
          tokenHash: createHash("sha256").update(legacyToken).digest("hex"),
          createdAt: "2026-08-26T00:00:00.000Z",
          requestId,
        },
      ],
      tokenRequests: [],
    }),
  );

  const child = spawn(process.execPath, ["server.mjs"], {
    cwd: import.meta.dirname,
    env: {
      ...process.env,
      PORT: String(port),
      HOST: "127.0.0.1",
      DATA_DIR: dataDir,
      COWORKPAL_USERS_JSON: "{}",
      COWORKPAL_ADMIN_TOKEN: adminToken,
    },
    stdio: "ignore",
  });
  context.after(() => child.kill());

  const base = `http://127.0.0.1:${port}`;
  await waitForHealth(base);
  const before = await adminOverview(base);
  assert.equal(before.users[0].tokens[0].status, "unavailable");

  const authenticated = await fetch(`${base}/v1/snapshot`, {
    headers: { Authorization: `Bearer ${legacyToken}` },
  });
  assert.equal(authenticated.status, 404);

  const after = await adminOverview(base);
  assert.equal(after.users[0].tokens[0].status, "active");
  assert.equal(after.users[0].tokens[0].accessToken, legacyToken);
  const persisted = readFileSync(join(dataDir, "auth-state.json"), "utf8");
  assert.equal(persisted.includes(legacyToken), false);
  assert.ok(JSON.parse(persisted).issuedUsers[0].encryptedToken);

  const recovery = await fetch(`${base}/v1/token-recoveries`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ userId: legacyUserId, deviceId: "replacement-device" }),
  });
  assert.equal(recovery.status, 201);
});

test("reuses a user for the same registered device and deletes all managed data", async (context) => {
  const dataDir = mkdtempSync(join(tmpdir(), "coworkpal-sync-user-delete-"));
  const port = 19500 + Math.floor(Math.random() * 400);
  const child = spawn(process.execPath, ["server.mjs"], {
    cwd: import.meta.dirname,
    env: {
      ...process.env,
      PORT: String(port),
      HOST: "127.0.0.1",
      DATA_DIR: dataDir,
      COWORKPAL_USERS_JSON: "{}",
      COWORKPAL_ADMIN_TOKEN: adminToken,
    },
    stdio: "ignore",
  });
  context.after(() => child.kill());

  const base = `http://127.0.0.1:${port}`;
  await waitForHealth(base);
  const first = await registerUser(base, "rog-cozyspot", "same-device");
  const snapshot = historySnapshot(1);
  snapshot.settings.catId = "same-device";
  assert.equal(
    (await fetch(`${base}/v1/snapshot`, {
      method: "PUT",
      headers: {
        Authorization: `Bearer ${first.accessToken}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify(snapshot),
    })).status,
    201,
  );

  const replacement = await registerUser(base, "rog-cozyspot", "same-device");
  assert.equal(replacement.userId, first.userId);
  assert.notEqual(replacement.accessToken, first.accessToken);
  assert.equal(
    (await fetch(`${base}/v1/snapshot`, {
      headers: { Authorization: `Bearer ${replacement.accessToken}` },
    })).status,
    200,
  );

  const overview = await adminOverview(base);
  assert.equal(overview.users.length, 1);
  assert.equal(overview.users[0].userId, first.userId);
  assert.equal(overview.users[0].tokens.length, 2);

  assert.equal(
    (await fetch(`${base}/v1/admin/users/${first.userId}`, { method: "DELETE" })).status,
    401,
  );
  const deleted = await fetch(`${base}/v1/admin/users/${first.userId}`, {
    method: "DELETE",
    headers: { Authorization: `Bearer ${adminToken}` },
  });
  assert.equal(deleted.status, 200);
  assert.equal((await deleted.json()).status, "deleted");
  assert.equal((await adminOverview(base)).users.length, 0);
  assert.equal(
    (await fetch(`${base}/v1/snapshot`, {
      headers: { Authorization: `Bearer ${replacement.accessToken}` },
    })).status,
    401,
  );
  assert.equal(existsSync(join(dataDir, "users", first.userId)), false);
  const persisted = JSON.parse(readFileSync(join(dataDir, "auth-state.json"), "utf8"));
  assert.equal(persisted.issuedUsers.length, 0);
  assert.equal(persisted.tokenRequests.some((request) => request.userId === first.userId), false);
  assert.equal(
    (await fetch(`${base}/v1/admin/users/${first.userId}`, {
      method: "DELETE",
      headers: { Authorization: `Bearer ${adminToken}` },
    })).status,
    404,
  );
});

test("keeps at most 30 snapshots and pushes a selected version for restore", async (context) => {
  const dataDir = mkdtempSync(join(tmpdir(), "coworkpal-sync-history-"));
  const port = 20000 + Math.floor(Math.random() * 1000);
  const historyToken = "history-token-that-is-longer-than-thirty-two-characters";
  const child = spawn(process.execPath, ["server.mjs"], {
    cwd: import.meta.dirname,
    env: {
      ...process.env,
      PORT: String(port),
      HOST: "127.0.0.1",
      DATA_DIR: dataDir,
      MAX_VERSIONS: "99",
      COWORKPAL_USERS_JSON: JSON.stringify({ history: historyToken }),
      COWORKPAL_ADMIN_TOKEN: adminToken,
    },
    stdio: "ignore",
  });
  context.after(() => child.kill());

  const base = `http://127.0.0.1:${port}`;
  await waitForHealth(base);
  let firstRevision;
  for (let index = 0; index < 31; index += 1) {
    const response = await fetch(`${base}/v1/snapshot`, {
      method: "PUT",
      headers: {
        Authorization: `Bearer ${historyToken}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify(historySnapshot(index)),
    });
    assert.equal(response.status, 201);
    const uploaded = await response.json();
    firstRevision ??= uploaded.revision;
  }

  const historyResponse = await fetch(`${base}/v1/snapshot/history`, {
    headers: { Authorization: `Bearer ${historyToken}` },
  });
  assert.equal(historyResponse.status, 200);
  const history = await historyResponse.json();
  assert.equal(history.versions.length, 30);
  assert.equal(history.versions.some((version) => version.revision === firstRevision), false);

  const overviewBefore = await adminOverview(base);
  const user = overviewBefore.users[0];
  assert.equal(user.historyCount, 30);
  assert.equal(user.versions.length, 30);
  assert.equal(user.pendingRestore, null);
  const selected = history.versions.at(-1);

  const unauthorizedRestore = await fetch(
    `${base}/v1/admin/users/${user.userId}/snapshots/${selected.revision}/restore`,
    { method: "POST" },
  );
  assert.equal(unauthorizedRestore.status, 401);
  const missingRestore = await fetch(
    `${base}/v1/admin/users/${user.userId}/snapshots/${"f".repeat(64)}/restore`,
    { method: "POST", headers: { Authorization: `Bearer ${adminToken}` } },
  );
  assert.equal(missingRestore.status, 404);

  const queueResponse = await fetch(
    `${base}/v1/admin/users/${user.userId}/snapshots/${selected.revision}/restore`,
    { method: "POST", headers: { Authorization: `Bearer ${adminToken}` } },
  );
  assert.equal(queueResponse.status, 202);
  const queued = await queueResponse.json();
  assert.equal(queued.status, "pending");
  assert.equal(queued.sourceRevision, selected.revision);

  assert.equal((await fetch(`${base}/v1/snapshot/restore-pending`)).status, 401);
  const pendingResponse = await fetch(`${base}/v1/snapshot/restore-pending`, {
    headers: { Authorization: `Bearer ${historyToken}` },
  });
  assert.equal(pendingResponse.status, 200);
  const pending = await pendingResponse.json();
  assert.equal(pending.restoreRequestId, queued.requestId);
  assert.equal(pending.sourceRevision, selected.revision);
  assert.equal(pending.data.exportedAt, selected.exportedAt);

  const latestResponse = await fetch(`${base}/v1/snapshot`, {
    headers: { Authorization: `Bearer ${historyToken}` },
  });
  const latest = await latestResponse.json();
  assert.equal(latest.revision, queued.revision);
  assert.equal(latest.data.exportedAt, selected.exportedAt);
  const overviewPending = await adminOverview(base);
  assert.equal(overviewPending.users[0].pendingRestore.requestId, queued.requestId);

  const wrongAck = await fetch(
    `${base}/v1/snapshot/restore-pending/00000000-0000-0000-0000-000000000000/ack`,
    { method: "POST", headers: { Authorization: `Bearer ${historyToken}` } },
  );
  assert.equal(wrongAck.status, 409);
  const ack = await fetch(
    `${base}/v1/snapshot/restore-pending/${queued.requestId}/ack`,
    { method: "POST", headers: { Authorization: `Bearer ${historyToken}` } },
  );
  assert.equal(ack.status, 200);
  assert.equal((await ack.json()).status, "restored");
  assert.equal(
    (await fetch(`${base}/v1/snapshot/restore-pending`, {
      headers: { Authorization: `Bearer ${historyToken}` },
    })).status,
    204,
  );
  const overviewAfter = await adminOverview(base);
  assert.equal(overviewAfter.users[0].historyCount, 30);
  assert.equal(overviewAfter.users[0].pendingRestore, null);
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

async function adminOverview(base) {
  const response = await fetch(`${base}/v1/admin/snapshots`, {
    headers: { Authorization: `Bearer ${adminToken}` },
  });
  assert.equal(response.status, 200);
  return response.json();
}

async function registerUser(base, userName, deviceId) {
  const requestResponse = await fetch(`${base}/v1/token-requests`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ userName, deviceId }),
  });
  assert.equal(requestResponse.status, 201);
  const request = await requestResponse.json();
  const approval = await fetch(`${base}/v1/admin/token-requests/${request.requestId}/approve`, {
    method: "POST",
    headers: { Authorization: `Bearer ${adminToken}` },
  });
  assert.equal(approval.status, 200);
  const claim = await fetch(`${base}/v1/token-requests/${request.requestId}`, {
    headers: { Authorization: `Bearer ${request.claimSecret}` },
  });
  assert.equal(claim.status, 200);
  return claim.json();
}

function findLatest(dataDir) {
  const userDir = join(dataDir, "users", readdirSync(join(dataDir, "users"))[0]);
  return join(userDir, "latest.json");
}

function historySnapshot(index) {
  return {
    schemaVersion: 1,
    exportedAt: index,
    appVersion: `history-${index}`,
    settings: { schemaVersion: 1, catId: "history-user" },
    workshop: { schemaVersion: 1, workshopLevel: 1 },
    layout: { schemaVersion: 1 },
    workLogs: { schemaVersion: 1, entries: {} },
    focusSessions: { schemaVersion: 1, sessions: [] },
    achievements: { schemaVersion: 1, unlocks: {} },
    notes: { schemaVersion: 1, notes: [] },
  };
}
