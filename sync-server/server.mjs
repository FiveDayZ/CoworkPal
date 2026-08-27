import {
  createCipheriv,
  createDecipheriv,
  createHash,
  randomBytes,
  randomUUID,
  timingSafeEqual,
} from "node:crypto";
import { createServer } from "node:http";
import {
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  renameSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { join } from "node:path";

const host = process.env.HOST ?? "0.0.0.0";
const port = Number(process.env.PORT ?? 18080);
const dataRoot = process.env.DATA_DIR ?? "/data";
const maxBodyBytes = Number(process.env.MAX_BODY_BYTES ?? 10 * 1024 * 1024);
const configuredMaxVersions = Number.parseInt(process.env.MAX_VERSIONS ?? "30", 10);
const maxVersions = Math.min(
  30,
  Math.max(1, Number.isFinite(configuredMaxVersions) ? configuredMaxVersions : 30),
);
const maxPendingRequests = Math.max(10, Number(process.env.MAX_PENDING_REQUESTS ?? 200));
const configuredUsers = loadUsers(process.env.COWORKPAL_USERS_JSON);
const authStatePath = join(dataRoot, "auth-state.json");
const adminToken = loadAdminToken(process.env.COWORKPAL_ADMIN_TOKEN);
const adminTokenHash = createHash("sha256").update(adminToken).digest();
const tokenEncryptionKey = createHash("sha256")
  .update(`coworkpal-user-token-v1:${adminToken}`)
  .digest();
let authState;
let users;
const requestAttempts = new Map();
const publicRoot = join(import.meta.dirname, "public");
const staticFiles = new Map([
  ["/", ["dashboard.html", "text/html; charset=utf-8"]],
  ["/dashboard.css", ["dashboard.css", "text/css; charset=utf-8"]],
  ["/dashboard.js", ["dashboard.js", "text/javascript; charset=utf-8"]],
  ["/cocat-avatar.png", ["cocat-avatar.png", "image/png"]],
]);

mkdirSync(dataRoot, { recursive: true });
authState = loadAuthState();
users = buildUsers();
validateAdminToken();

const server = createServer((request, response) => {
  void handleRequest(request, response).catch((error) => {
    const statusCode = Number(error?.statusCode) || 500;
    if (statusCode >= 500) console.error("request failed", error);
    sendJson(response, statusCode, {
      error: statusCode >= 500 ? "internal server error" : error.message,
    });
  });
});

server.listen(port, host, () => {
  console.log(`CoworkPal sync server listening on ${host}:${port}`);
});

async function handleRequest(request, response) {
  setSecurityHeaders(response);
  const requestUrl = new URL(request.url, "http://localhost");

  if (request.method === "GET" && staticFiles.has(requestUrl.pathname)) {
    sendStaticFile(response, requestUrl.pathname);
    return;
  }

  if (request.method === "GET" && requestUrl.pathname === "/health") {
    sendJson(response, 200, { status: "ok" });
    return;
  }

  if (request.method === "GET" && requestUrl.pathname === "/v1/admin/snapshots") {
    if (!authenticateAdmin(request.headers.authorization)) {
      response.setHeader("WWW-Authenticate", "Bearer");
      sendJson(response, 401, { error: "invalid admin token" });
      return;
    }
    sendAdminSnapshots(response);
    return;
  }

  if (request.method === "POST" && requestUrl.pathname === "/v1/token-requests") {
    enforceRequestRateLimit(request.socket.remoteAddress ?? "unknown");
    const body = await readJsonBody(request);
    sendJson(response, 201, createTokenRequest(body));
    return;
  }

  if (request.method === "POST" && requestUrl.pathname === "/v1/token-recoveries") {
    enforceRequestRateLimit(request.socket.remoteAddress ?? "unknown");
    const body = await readJsonBody(request);
    sendJson(response, 201, createTokenRecovery(body));
    return;
  }

  const tokenRequestMatch = requestUrl.pathname.match(/^\/v1\/token-requests\/([0-9a-f-]+)$/i);
  if (request.method === "GET" && tokenRequestMatch) {
    sendTokenRequestStatus(response, tokenRequestMatch[1], request.headers.authorization);
    return;
  }

  const adminRequestMatch = requestUrl.pathname.match(
    /^\/v1\/admin\/token-requests\/([0-9a-f-]+)\/(approve|reject)$/i,
  );
  if (request.method === "POST" && adminRequestMatch) {
    if (!authenticateAdmin(request.headers.authorization)) {
      response.setHeader("WWW-Authenticate", "Bearer");
      sendJson(response, 401, { error: "invalid admin token" });
      return;
    }
    decideTokenRequest(response, adminRequestMatch[1], adminRequestMatch[2]);
    return;
  }

  const adminTokenMatch = requestUrl.pathname.match(
    /^\/v1\/admin\/users\/([0-9a-f-]+)\/tokens\/([0-9a-f-]+)\/(rotate|revoke)$/i,
  );
  if (request.method === "POST" && adminTokenMatch) {
    if (!authenticateAdmin(request.headers.authorization)) {
      response.setHeader("WWW-Authenticate", "Bearer");
      sendJson(response, 401, { error: "invalid admin token" });
      return;
    }
    manageUserToken(response, adminTokenMatch[1], adminTokenMatch[2], adminTokenMatch[3]);
    return;
  }

  const adminUserMatch = requestUrl.pathname.match(
    /^\/v1\/admin\/users\/([0-9a-f-]{32,64})$/i,
  );
  if (request.method === "DELETE" && adminUserMatch) {
    if (!authenticateAdmin(request.headers.authorization)) {
      response.setHeader("WWW-Authenticate", "Bearer");
      sendJson(response, 401, { error: "invalid admin token" });
      return;
    }
    deleteUser(response, adminUserMatch[1]);
    return;
  }

  const adminRestoreMatch = requestUrl.pathname.match(
    /^\/v1\/admin\/users\/([0-9a-f-]{32,64})\/snapshots\/([0-9a-f-]{32,100})\/restore$/i,
  );
  if (request.method === "POST" && adminRestoreMatch) {
    if (!authenticateAdmin(request.headers.authorization)) {
      response.setHeader("WWW-Authenticate", "Bearer");
      sendJson(response, 401, { error: "invalid admin token" });
      return;
    }
    queueSnapshotRestore(response, adminRestoreMatch[1], adminRestoreMatch[2]);
    return;
  }

  if (request.method === "GET" && requestUrl.pathname === "/v1/snapshot/restore-pending") {
    const user = authenticate(request.headers.authorization);
    if (!user) {
      response.setHeader("WWW-Authenticate", "Bearer");
      sendJson(response, 401, { error: "invalid access token" });
      return;
    }
    sendPendingRestore(response, user);
    return;
  }

  const restoreAckMatch = requestUrl.pathname.match(
    /^\/v1\/snapshot\/restore-pending\/([0-9a-f-]{36})\/ack$/i,
  );
  if (request.method === "POST" && restoreAckMatch) {
    const user = authenticate(request.headers.authorization);
    if (!user) {
      response.setHeader("WWW-Authenticate", "Bearer");
      sendJson(response, 401, { error: "invalid access token" });
      return;
    }
    acknowledgePendingRestore(response, user, restoreAckMatch[1]);
    return;
  }

  if (request.method === "GET" && requestUrl.pathname === "/v1/snapshot/history") {
    const user = authenticate(request.headers.authorization);
    if (!user) {
      response.setHeader("WWW-Authenticate", "Bearer");
      sendJson(response, 401, { error: "invalid access token" });
      return;
    }
    sendSnapshotHistory(response, user);
    return;
  }

  if (requestUrl.pathname !== "/v1/snapshot") {
    sendJson(response, 404, { error: "not found" });
    return;
  }

  const user = authenticate(request.headers.authorization);
  if (!user) {
    response.setHeader("WWW-Authenticate", "Bearer");
    sendJson(response, 401, { error: "invalid access token" });
    return;
  }

  if (request.method === "GET") {
    sendLatest(response, user);
    return;
  }

  if (request.method === "PUT") {
    const snapshot = await readJsonBody(request);
    validateSnapshot(snapshot);
    const envelope = storeSnapshot(user, snapshot);
    sendJson(response, 201, {
      revision: envelope.revision,
      storedAt: envelope.storedAt,
    });
    return;
  }

  response.setHeader("Allow", "GET, PUT");
  sendJson(response, 405, { error: "method not allowed" });
}

function loadUsers(raw) {
  if (!raw) {
    throw new Error("COWORKPAL_USERS_JSON is required");
  }

  const parsed = JSON.parse(raw);
  const entries = Object.entries(parsed);
  return entries.map(([name, token]) => {
    if (!name.trim() || typeof token !== "string" || token.length < 32) {
      throw new Error("each user needs a name and an access token of at least 32 characters");
    }
    return {
      id: createHash("sha256").update(name).digest("hex"),
      name,
      tokenId: createHash("sha256").update(`configured:${name}`).digest("hex"),
      tokenHash: createHash("sha256").update(token).digest(),
      accessToken: token,
      source: "configured",
      revoked: false,
    };
  });
}

function loadAuthState() {
  try {
    const parsed = JSON.parse(readFileSync(authStatePath, "utf8"));
    if (!Array.isArray(parsed.issuedUsers) || !Array.isArray(parsed.tokenRequests)) {
      throw new Error("invalid auth state shape");
    }
    return parsed;
  } catch (error) {
    if (error.code === "ENOENT") return { schemaVersion: 1, issuedUsers: [], tokenRequests: [] };
    throw new Error(`could not load ${authStatePath}: ${error.message}`);
  }
}

function buildUsers() {
  const issued = authState.issuedUsers.map((user) => ({
    id: user.id,
    name: user.name,
    tokenId: user.tokenId ?? user.requestId,
    tokenHash: Buffer.from(user.tokenHash, "hex"),
    requestId: user.requestId,
    accessToken: decryptAccessToken(user.encryptedToken),
    source: "issued",
    createdAt: user.createdAt,
    updatedAt: user.updatedAt,
    revoked: Boolean(user.revoked),
  }));
  return [...configuredUsers, ...issued];
}

function persistAuthState(nextState) {
  const temporaryPath = `${authStatePath}.${randomUUID()}.tmp`;
  writeFileSync(temporaryPath, JSON.stringify(nextState), { encoding: "utf8", mode: 0o600 });
  renameSync(temporaryPath, authStatePath);
  authState = nextState;
  users = buildUsers();
}

function loadAdminToken(token) {
  if (typeof token !== "string" || token.length < 32) {
    throw new Error("COWORKPAL_ADMIN_TOKEN must be at least 32 characters");
  }
  return token;
}

function validateAdminToken() {
  if (users.some((user) => timingSafeEqual(user.tokenHash, adminTokenHash))) {
    throw new Error("COWORKPAL_ADMIN_TOKEN must differ from every user token");
  }
}

function authenticate(authorization) {
  if (!authorization?.startsWith("Bearer ")) return null;
  const accessToken = authorization.slice("Bearer ".length);
  const candidate = createHash("sha256")
    .update(accessToken)
    .digest();
  const user = users.find(
    (candidateUser) => !candidateUser.revoked && timingSafeEqual(candidate, candidateUser.tokenHash),
  );
  if (user) rememberIssuedAccessToken(user, accessToken);
  return user ?? null;
}

function rememberIssuedAccessToken(user, accessToken) {
  if (user.source !== "issued" || user.accessToken) return;
  const nextState = structuredClone(authState);
  const stored = nextState.issuedUsers.find(
    (candidate) =>
      candidate.id === user.id &&
      (candidate.tokenId ?? candidate.requestId) === user.tokenId &&
      !candidate.encryptedToken,
  );
  if (!stored) return;
  stored.encryptedToken = encryptAccessToken(accessToken);
  stored.updatedAt = new Date().toISOString();
  try {
    persistAuthState(nextState);
  } catch (error) {
    console.error(`could not preserve access token for user ${user.id}`, error);
  }
}

function authenticateAdmin(authorization) {
  if (!authorization?.startsWith("Bearer ")) return false;
  const candidate = createHash("sha256")
    .update(authorization.slice("Bearer ".length))
    .digest();
  return timingSafeEqual(candidate, adminTokenHash);
}

function createTokenRequest(body) {
  const userName = typeof body?.userName === "string" ? body.userName.trim() : "";
  const deviceId = typeof body?.deviceId === "string" ? body.deviceId.trim() : "";
  if (!userName || userName.length > 40 || /[\u0000-\u001f\u007f]/.test(userName)) {
    throw badRequest("userName must contain 1 to 40 visible characters");
  }
  if (deviceId.length > 100 || /[\u0000-\u001f\u007f]/.test(deviceId)) {
    throw badRequest("deviceId must contain at most 100 visible characters");
  }
  ensurePendingCapacity();

  const claimSecret = randomBytes(32).toString("hex");
  const item = {
    id: randomUUID(),
    kind: "registration",
    userId: null,
    userName,
    deviceId,
    requestedAt: new Date().toISOString(),
    decidedAt: null,
    status: "pending",
    claimSecretHash: sha256Hex(claimSecret),
    issuedToken: null,
  };
  const nextState = structuredClone(authState);
  nextState.tokenRequests.push(item);
  persistAuthState(nextState);
  return publicTokenRequest(item, { claimSecret });
}

function createTokenRecovery(body) {
  const userId = typeof body?.userId === "string" ? body.userId.trim() : "";
  const deviceId = typeof body?.deviceId === "string" ? body.deviceId.trim() : "";
  if (!/^[0-9a-f-]{32,64}$/i.test(userId)) {
    throw badRequest("userId is invalid");
  }
  if (deviceId.length > 100 || /[\u0000-\u001f\u007f]/.test(deviceId)) {
    throw badRequest("deviceId must contain at most 100 visible characters");
  }
  const user = users.find((candidate) => candidate.id === userId && !candidate.revoked);
  if (!user) throw badRequest("userId is unknown or disabled");
  if (!recoverableToken(userId)) {
    throw badRequest("this user has no recoverable token; ask the administrator to rotate it");
  }
  ensurePendingCapacity();

  const claimSecret = randomBytes(32).toString("hex");
  const item = {
    id: randomUUID(),
    kind: "recovery",
    userId,
    userName: user.name,
    deviceId,
    requestedAt: new Date().toISOString(),
    decidedAt: null,
    status: "pending",
    claimSecretHash: sha256Hex(claimSecret),
    issuedToken: null,
  };
  const nextState = structuredClone(authState);
  nextState.tokenRequests.push(item);
  persistAuthState(nextState);
  return publicTokenRequest(item, { claimSecret });
}

function ensurePendingCapacity() {
  const pendingCount = authState.tokenRequests.filter((item) => item.status === "pending").length;
  if (pendingCount >= maxPendingRequests) {
    const error = new Error("too many token requests");
    error.statusCode = 503;
    throw error;
  }
}

function sendTokenRequestStatus(response, requestId, authorization) {
  const item = authState.tokenRequests.find((request) => request.id === requestId);
  if (!item || !matchesSecret(authorization, item.claimSecretHash)) {
    response.setHeader("WWW-Authenticate", "Bearer");
    sendJson(response, 401, { error: "invalid token request credentials" });
    return;
  }
  sendJson(response, 200, publicTokenRequest(item, { accessToken: item.issuedToken }));
}

function decideTokenRequest(response, requestId, decision) {
  const nextState = structuredClone(authState);
  const item = nextState.tokenRequests.find((request) => request.id === requestId);
  if (!item) {
    sendJson(response, 404, { error: "token request not found" });
    return;
  }
  if (item.status !== "pending") {
    sendJson(response, 409, { error: "token request was already decided" });
    return;
  }

  item.status = decision === "approve" ? "approved" : "rejected";
  item.decidedAt = new Date().toISOString();
  if (item.status === "approved") {
    if (item.kind === "recovery") {
      item.issuedToken = recoverableToken(item.userId);
      if (!item.issuedToken) {
        sendJson(response, 409, { error: "user token is no longer recoverable" });
        return;
      }
    } else {
      const accessToken = randomBytes(32).toString("hex");
      const userId = reusableIssuedUserId(item.userName, item.deviceId) ?? randomUUID();
      nextState.issuedUsers.push({
        id: userId,
        name: item.userName,
        tokenId: item.id,
        tokenHash: sha256Hex(accessToken),
        encryptedToken: encryptAccessToken(accessToken),
        createdAt: item.decidedAt,
        updatedAt: item.decidedAt,
        requestId: item.id,
        revoked: false,
      });
      item.userId = userId;
      item.issuedToken = accessToken;
    }
  }
  persistAuthState(nextState);
  sendJson(response, 200, publicTokenRequest(item));
}

function publicTokenRequest(item, secrets = {}) {
  return {
    requestId: item.id,
    kind: item.kind ?? "registration",
    userId: item.userId ?? null,
    userName: item.userName,
    deviceId: item.deviceId,
    requestedAt: item.requestedAt,
    decidedAt: item.decidedAt,
    status: item.status,
    ...(secrets.claimSecret ? { claimSecret: secrets.claimSecret } : {}),
    ...(secrets.accessToken ? { accessToken: secrets.accessToken } : {}),
  };
}

function encryptAccessToken(token) {
  if (!token) return null;
  const iv = randomBytes(12);
  const cipher = createCipheriv("aes-256-gcm", tokenEncryptionKey, iv);
  const ciphertext = Buffer.concat([cipher.update(token, "utf8"), cipher.final()]);
  return {
    algorithm: "aes-256-gcm",
    iv: iv.toString("base64"),
    tag: cipher.getAuthTag().toString("base64"),
    ciphertext: ciphertext.toString("base64"),
  };
}

function decryptAccessToken(encrypted) {
  if (!encrypted || encrypted.algorithm !== "aes-256-gcm") return null;
  try {
    const decipher = createDecipheriv(
      "aes-256-gcm",
      tokenEncryptionKey,
      Buffer.from(encrypted.iv, "base64"),
    );
    decipher.setAuthTag(Buffer.from(encrypted.tag, "base64"));
    return Buffer.concat([
      decipher.update(Buffer.from(encrypted.ciphertext, "base64")),
      decipher.final(),
    ]).toString("utf8");
  } catch {
    return null;
  }
}

function recoverableToken(userId) {
  const candidates = users.filter((user) => user.id === userId && !user.revoked && user.accessToken);
  return candidates.at(-1)?.accessToken ?? null;
}

function reusableIssuedUserId(userName, deviceId) {
  if (!deviceId) return null;
  const candidateIds = new Set(
    authState.issuedUsers
      .filter((user) => user.name === userName)
      .map((user) => user.id),
  );
  let newestMatch = null;
  for (const userId of candidateIds) {
    try {
      const envelope = JSON.parse(
        readFileSync(join(dataRoot, "users", userId, "latest.json"), "utf8"),
      );
      if (String(envelope.data?.settings?.catId ?? "") !== deviceId) continue;
      const storedAt = String(envelope.storedAt ?? "");
      if (!newestMatch || storedAt > newestMatch.storedAt) newestMatch = { userId, storedAt };
    } catch (error) {
      if (error.code !== "ENOENT") {
        console.error(`could not inspect registration target ${userId}`, error);
      }
    }
  }
  return newestMatch?.userId ?? null;
}

function manageUserToken(response, userId, tokenId, action) {
  const nextState = structuredClone(authState);
  const token = nextState.issuedUsers.find(
    (candidate) => candidate.id === userId && (candidate.tokenId ?? candidate.requestId) === tokenId,
  );
  if (!token) {
    sendJson(response, 404, { error: "managed token not found" });
    return;
  }

  if (action === "revoke") {
    token.revoked = true;
    token.updatedAt = new Date().toISOString();
    persistAuthState(nextState);
    sendJson(response, 200, { userId, tokenId, status: "revoked" });
    return;
  }

  const accessToken = randomBytes(32).toString("hex");
  token.tokenHash = sha256Hex(accessToken);
  token.encryptedToken = encryptAccessToken(accessToken);
  token.revoked = false;
  token.updatedAt = new Date().toISOString();
  persistAuthState(nextState);
  sendJson(response, 200, { userId, tokenId, status: "active", accessToken });
}

function deleteUser(response, userId) {
  if (configuredUsers.some((user) => user.id === userId)) {
    sendJson(response, 409, {
      error: "configured users must be removed from COWORKPAL_USERS_JSON",
    });
    return;
  }
  if (!authState.issuedUsers.some((user) => user.id === userId)) {
    sendJson(response, 404, { error: "managed user not found" });
    return;
  }

  const nextState = structuredClone(authState);
  nextState.issuedUsers = nextState.issuedUsers.filter((user) => user.id !== userId);
  nextState.tokenRequests = nextState.tokenRequests.filter((request) => request.userId !== userId);

  const directory = join(dataRoot, "users", userId);
  const deletedDirectory = `${directory}.deleting-${randomUUID()}`;
  const hasDirectory = existsSync(directory);
  if (hasDirectory) renameSync(directory, deletedDirectory);
  try {
    persistAuthState(nextState);
  } catch (error) {
    if (hasDirectory) renameSync(deletedDirectory, directory);
    throw error;
  }
  if (hasDirectory) {
    try {
      rmSync(deletedDirectory, { recursive: true, force: true });
    } catch (error) {
      console.error(`could not remove deleted user data ${userId}`, error);
    }
  }
  sendJson(response, 200, { userId, status: "deleted" });
}

function matchesSecret(authorization, expectedHash) {
  if (!authorization?.startsWith("Bearer ")) return false;
  const candidate = Buffer.from(sha256Hex(authorization.slice("Bearer ".length)), "hex");
  const expected = Buffer.from(expectedHash, "hex");
  return candidate.length === expected.length && timingSafeEqual(candidate, expected);
}

function sha256Hex(value) {
  return createHash("sha256").update(value).digest("hex");
}

function enforceRequestRateLimit(address) {
  const now = Date.now();
  const windowStart = now - 10 * 60 * 1000;
  const attempts = (requestAttempts.get(address) ?? []).filter((timestamp) => timestamp > windowStart);
  if (attempts.length >= 5) {
    const error = new Error("too many token requests, try again later");
    error.statusCode = 429;
    throw error;
  }
  attempts.push(now);
  requestAttempts.set(address, attempts);
}

async function readJsonBody(request) {
  const chunks = [];
  let length = 0;
  for await (const chunk of request) {
    length += chunk.length;
    if (length > maxBodyBytes) {
      const error = new Error("request body too large");
      error.statusCode = 413;
      throw error;
    }
    chunks.push(chunk);
  }

  try {
    return JSON.parse(Buffer.concat(chunks).toString("utf8"));
  } catch {
    const error = new Error("invalid JSON body");
    error.statusCode = 400;
    throw error;
  }
}

function validateSnapshot(snapshot) {
  const required = [
    "settings",
    "workshop",
    "layout",
    "workLogs",
    "focusSessions",
    "achievements",
    "notes",
  ];
  if (!snapshot || typeof snapshot !== "object" || Array.isArray(snapshot)) {
    throw badRequest("snapshot must be an object");
  }
  for (const key of required) {
    if (!snapshot[key] || typeof snapshot[key] !== "object" || Array.isArray(snapshot[key])) {
      throw badRequest(`snapshot.${key} must be an object`);
    }
  }
  if (snapshot.schemaVersion !== 1) {
    throw badRequest("unsupported snapshot schemaVersion");
  }
}

function badRequest(message) {
  const error = new Error(message);
  error.statusCode = 400;
  return error;
}

function userDirectory(user) {
  return join(dataRoot, "users", user.id);
}

function pendingRestorePath(user) {
  return join(userDirectory(user), "pending-restore.json");
}

function sendAdminSnapshots(response) {
  const groupedUsers = new Map();
  for (const user of users) {
    const group = groupedUsers.get(user.id) ?? { id: user.id, name: user.name, tokens: [] };
    group.tokens.push(user);
    groupedUsers.set(user.id, group);
  }
  const summaries = [...groupedUsers.values()].map((user) => {
    const versions = snapshotHistory(user);
    const pendingRestore = pendingRestoreSummary(user);
    try {
      const content = readFileSync(join(userDirectory(user), "latest.json"), "utf8");
      const envelope = JSON.parse(content);
      return {
        userId: user.id,
        userName: user.name,
        tokens: user.tokens.map(adminTokenSummary),
        historyCount: versions.length,
        versions,
        pendingRestore,
        backup: summarizeBackup(envelope, Buffer.byteLength(content)),
      };
    } catch (error) {
      if (error.code === "ENOENT") {
        return {
          userId: user.id,
          userName: user.name,
          tokens: user.tokens.map(adminTokenSummary),
          historyCount: 0,
          versions: [],
          pendingRestore,
          backup: null,
        };
      }
      console.error(`could not summarize backup for user ${user.id}`, error);
      return {
        userId: user.id,
        userName: user.name,
        tokens: user.tokens.map(adminTokenSummary),
        historyCount: 0,
        versions: [],
        pendingRestore,
        backup: { status: "invalid" },
      };
    }
  });

  sendJson(response, 200, {
    generatedAt: new Date().toISOString(),
    tokenRequests: authState.tokenRequests
      .map((item) => publicTokenRequest(item))
      .sort((left, right) => right.requestedAt.localeCompare(left.requestedAt)),
    users: summaries,
  });
}

function adminTokenSummary(token) {
  return {
    tokenId: token.tokenId,
    accessToken: token.accessToken,
    source: token.source,
    status: token.revoked ? "revoked" : token.accessToken ? "active" : "unavailable",
    createdAt: token.createdAt ?? null,
    updatedAt: token.updatedAt ?? null,
  };
}

function summarizeBackup(envelope, sizeBytes) {
  const data = envelope?.data ?? {};
  const settings = data.settings ?? {};
  const workshop = data.workshop ?? {};
  const profile = data.deviceProfile ?? {};
  const inventory = profile.inventory ?? {};

  return {
    status: "ready",
    revision: String(envelope.revision ?? ""),
    storedAt: String(envelope.storedAt ?? ""),
    exportedAt: Number(data.exportedAt ?? 0),
    appVersion: String(data.appVersion ?? "unknown"),
    sizeBytes,
    device: {
      catId: String(settings.catId ?? "unknown"),
      themeName: String(settings.themeName ?? "coworkpal"),
      launchAtStartup: Boolean(settings.launchAtStartup),
      lowPowerMode: Boolean(settings.enableLowPowerMode),
      staticCatMode: Boolean(settings.enableStaticCatMode),
      sleepMode: Boolean(settings.enableSleepMode),
      notifications: Boolean(settings.enableNotifications),
      hardwareMonitor: Boolean(settings.integratedHardwareMonitorEnabled),
      samplingIntervalMs: Number(settings.samplingIntervalMs ?? 0),
      backgroundSamplingIntervalMs: Number(settings.backgroundSamplingIntervalMs ?? 0),
      monitorBarMode: String(settings.monitorBarMode ?? "Default"),
      visibleMonitorMetrics: Array.isArray(settings.visibleMonitorMetrics)
        ? settings.visibleMonitorMetrics.map(String)
        : [],
    },
    hardware: {
      capturedAt: Number(profile.capturedAt ?? 0),
      cpuName: optionalString(profile.cpuName),
      gpuName: optionalString(profile.gpuName),
      cpuPhysicalCoreCount: optionalNumber(profile.cpuPhysicalCoreCount),
      cpuLogicalCoreCount: optionalNumber(profile.cpuLogicalCoreCount),
      totalMemoryBytes: optionalNumber(profile.totalMemoryBytes),
      gpuMemoryTotalBytes: optionalNumber(profile.gpuMemoryTotalBytes),
      motherboard: deviceList(inventory.motherboard),
      memoryModules: memoryModuleList(inventory.memoryModules),
      gpus: deviceList(inventory.gpus),
      displays: deviceList(inventory.displays),
      disks: deviceList(inventory.disks),
      audioDevices: deviceList(inventory.audioDevices),
      networkAdapters: deviceList(inventory.networkAdapters),
    },
    workshop: {
      level: Number(workshop.workshopLevel ?? 1),
      affinityLevel: Number(workshop.catAffinityLevel ?? 1),
      totalOnlineSeconds: Number(workshop.totalOnlineSeconds ?? 0),
      parts: Number(workshop.parts ?? 0),
      insight: Number(workshop.insight ?? 0),
      todayParts: Number(workshop.todayParts ?? 0),
      todayInsight: Number(workshop.todayInsight ?? 0),
      moduleLevels: workshop.moduleLevels ?? {},
    },
    counts: {
      notes: arrayLength(data.notes?.notes),
      focusSessions: arrayLength(data.focusSessions?.sessions),
      workLogDays: objectSize(data.workLogs?.entries),
      achievements: objectSize(data.achievements?.unlocks),
    },
  };
}

function optionalString(value) {
  return typeof value === "string" && value.trim() ? value : null;
}

function optionalNumber(value) {
  const number = Number(value);
  return Number.isFinite(number) && number >= 0 ? number : null;
}

function deviceList(value) {
  if (!Array.isArray(value)) return [];
  return value.map((item) => ({
    name: String(item?.name ?? "unknown"),
    detail: optionalString(item?.detail),
    vendor: optionalString(item?.vendor),
    capacityBytes: optionalNumber(item?.capacityBytes),
  }));
}

function memoryModuleList(value) {
  if (!Array.isArray(value)) return [];
  return value.map((item) => ({
    manufacturer: optionalString(item?.manufacturer),
    partNumber: optionalString(item?.partNumber),
    capacityBytes: optionalNumber(item?.capacityBytes),
    speedMhz: optionalNumber(item?.speedMhz),
  }));
}

function arrayLength(value) {
  return Array.isArray(value) ? value.length : 0;
}

function objectSize(value) {
  return value && typeof value === "object" && !Array.isArray(value)
    ? Object.keys(value).length
    : 0;
}

function sendLatest(response, user) {
  try {
    const content = readFileSync(join(userDirectory(user), "latest.json"));
    response.writeHead(200, { "Content-Type": "application/json; charset=utf-8" });
    response.end(content);
  } catch (error) {
    if (error.code === "ENOENT") {
      sendJson(response, 404, { error: "no cloud snapshot exists" });
      return;
    }
    throw error;
  }
}

function snapshotHistory(user) {
  const directory = join(userDirectory(user), "versions");
  try {
    return readdirSync(directory)
      .filter((name) => name.endsWith(".json"))
      .sort()
      .reverse()
      .map((name) => {
        const content = readFileSync(join(directory, name), "utf8");
        const envelope = JSON.parse(content);
        return {
          revision: String(envelope.revision ?? name.slice(0, -5)),
          storedAt: String(envelope.storedAt ?? ""),
          exportedAt: Number(envelope.data?.exportedAt ?? 0),
          appVersion: String(envelope.data?.appVersion ?? "unknown"),
          sizeBytes: Buffer.byteLength(content),
        };
      });
  } catch (error) {
    if (error.code === "ENOENT") return [];
    throw error;
  }
}

function sendSnapshotHistory(response, user) {
  sendJson(response, 200, { userId: user.id, versions: snapshotHistory(user) });
}

function readSnapshotVersion(user, revision) {
  if (!/^[0-9a-f-]{32,100}$/i.test(revision)) throw badRequest("snapshot revision is invalid");
  try {
    const content = readFileSync(join(userDirectory(user), "versions", `${revision}.json`), "utf8");
    return JSON.parse(content);
  } catch (error) {
    if (error.code === "ENOENT") {
      const missing = new Error("snapshot version not found");
      missing.statusCode = 404;
      throw missing;
    }
    throw error;
  }
}

function readPendingRestore(user) {
  try {
    return JSON.parse(readFileSync(pendingRestorePath(user), "utf8"));
  } catch (error) {
    if (error.code === "ENOENT") return null;
    throw error;
  }
}

function pendingRestoreSummary(user) {
  const pending = readPendingRestore(user);
  if (!pending) return null;
  return {
    requestId: pending.requestId,
    sourceRevision: pending.sourceRevision,
    revision: pending.envelope.revision,
    requestedAt: pending.requestedAt,
  };
}

function queueSnapshotRestore(response, userId, sourceRevision) {
  const user = users.find((candidate) => candidate.id === userId && !candidate.revoked);
  if (!user) {
    sendJson(response, 404, { error: "active user not found" });
    return;
  }
  const selected = readSnapshotVersion(user, sourceRevision);
  validateSnapshot(selected.data);
  const envelope = createSnapshotEnvelope(selected.data);
  writeSnapshotEnvelope(user, envelope);

  const pending = {
    requestId: randomUUID(),
    sourceRevision,
    requestedAt: new Date().toISOString(),
    envelope,
  };
  const temporaryPath = `${pendingRestorePath(user)}.${randomUUID()}.tmp`;
  writeFileSync(temporaryPath, JSON.stringify(pending), { encoding: "utf8", mode: 0o600 });
  renameSync(temporaryPath, pendingRestorePath(user));
  sendJson(response, 202, { userId, ...pendingRestoreSummary(user), status: "pending" });
}

function sendPendingRestore(response, user) {
  const pending = readPendingRestore(user);
  if (!pending) {
    response.writeHead(204);
    response.end();
    return;
  }
  sendJson(response, 200, {
    restoreRequestId: pending.requestId,
    sourceRevision: pending.sourceRevision,
    requestedAt: pending.requestedAt,
    ...pending.envelope,
  });
}

function acknowledgePendingRestore(response, user, requestId) {
  const pending = readPendingRestore(user);
  if (!pending) {
    sendJson(response, 404, { error: "no pending restore exists" });
    return;
  }
  if (pending.requestId !== requestId) {
    sendJson(response, 409, { error: "pending restore was replaced by a newer request" });
    return;
  }
  writeSnapshotEnvelope(user, pending.envelope);
  rmSync(pendingRestorePath(user));
  sendJson(response, 200, { requestId, status: "restored" });
}

function createSnapshotEnvelope(data) {
  return {
    revision: `${Date.now()}-${randomUUID()}`,
    storedAt: new Date().toISOString(),
    data,
  };
}

function writeSnapshotEnvelope(user, envelope) {
  const directory = userDirectory(user);
  const versionsDirectory = join(directory, "versions");
  mkdirSync(versionsDirectory, { recursive: true });
  const content = JSON.stringify(envelope);
  const versionPath = join(versionsDirectory, `${envelope.revision}.json`);
  const temporaryPath = join(directory, `latest.${randomUUID()}.tmp`);
  writeFileSync(versionPath, content, { encoding: "utf8", mode: 0o600 });
  writeFileSync(temporaryPath, content, { encoding: "utf8", mode: 0o600 });
  renameSync(temporaryPath, join(directory, "latest.json"));
  pruneVersions(versionsDirectory);
}

function storeSnapshot(user, data) {
  const envelope = createSnapshotEnvelope(data);
  writeSnapshotEnvelope(user, envelope);
  markTokenRequestClaimed(user.requestId, user.id);
  return envelope;
}

function markTokenRequestClaimed(requestId, userId) {
  const claimable = authState.tokenRequests.filter(
    (item) =>
      item.issuedToken &&
      (item.id === requestId || (item.kind === "recovery" && item.userId === userId)),
  );
  if (claimable.length === 0) return;
  const nextState = structuredClone(authState);
  const claimedAt = new Date().toISOString();
  for (const item of nextState.tokenRequests) {
    if (claimable.some((candidate) => candidate.id === item.id)) {
      item.issuedToken = null;
      item.claimedAt = claimedAt;
    }
  }
  try {
    persistAuthState(nextState);
  } catch (error) {
    console.error(`could not mark token requests for user ${userId} as claimed`, error);
  }
}

function pruneVersions(directory) {
  const versions = readdirSync(directory)
    .filter((name) => name.endsWith(".json"))
    .sort()
    .reverse();
  for (const oldVersion of versions.slice(maxVersions)) {
    rmSync(join(directory, oldVersion));
  }
}

function setSecurityHeaders(response) {
  response.setHeader("Cache-Control", "no-store");
  response.setHeader("X-Content-Type-Options", "nosniff");
  response.setHeader("Referrer-Policy", "no-referrer");
  response.setHeader(
    "Content-Security-Policy",
    "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self'; connect-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'",
  );
  response.setHeader("Permissions-Policy", "camera=(), microphone=(), geolocation=()");
}

function sendStaticFile(response, pathname) {
  const [fileName, contentType] = staticFiles.get(pathname);
  const content = readFileSync(join(publicRoot, fileName));
  response.writeHead(200, { "Content-Type": contentType });
  response.end(content);
}

function sendJson(response, statusCode, body) {
  response.writeHead(statusCode, { "Content-Type": "application/json; charset=utf-8" });
  response.end(JSON.stringify(body));
}
