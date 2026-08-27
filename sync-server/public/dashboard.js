const loginLayer = document.querySelector("#login-layer");
const loginForm = document.querySelector("#login-form");
const tokenInput = document.querySelector("#admin-token");
const toggleTokenButton = document.querySelector("#toggle-token");
const loginError = document.querySelector("#login-error");
const appShell = document.querySelector("#app-shell");
const refreshButton = document.querySelector("#refresh-button");
const logoutButton = document.querySelector("#logout-button");
const retryButton = document.querySelector("#retry-button");
const searchInput = document.querySelector("#search-input");
const loadingState = document.querySelector("#loading-state");
const errorState = document.querySelector("#error-state");
const errorMessage = document.querySelector("#error-message");
const emptyState = document.querySelector("#empty-state");
const userList = document.querySelector("#user-list");
const resultCount = document.querySelector("#result-count");
const freshness = document.querySelector("#freshness");
const requestList = document.querySelector("#request-list");
const requestEmpty = document.querySelector("#request-empty");
const requestCount = document.querySelector("#request-count");

const state = {
  token: sessionStorage.getItem("coworkpal-admin-token") ?? "",
  tokenRequests: [],
  users: [],
  generatedAt: "",
};

loginForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  const token = tokenInput.value.trim();
  if (token.length < 32) {
    loginError.textContent = "管理员令牌至少需要 32 个字符。";
    return;
  }
  state.token = token;
  setLoginPending(true);
  const loaded = await loadOverview({ loginAttempt: true });
  setLoginPending(false);
  if (loaded) {
    sessionStorage.setItem("coworkpal-admin-token", token);
    showDashboard();
  }
});

toggleTokenButton.addEventListener("click", () => {
  const reveal = tokenInput.type === "password";
  tokenInput.type = reveal ? "text" : "password";
  toggleTokenButton.title = reveal ? "隐藏令牌" : "显示令牌";
  toggleTokenButton.setAttribute("aria-label", toggleTokenButton.title);
});

refreshButton.addEventListener("click", () => loadOverview());
retryButton.addEventListener("click", () => loadOverview());
logoutButton.addEventListener("click", () => logout());
searchInput.addEventListener("input", renderUsers);

if (state.token) {
  loadOverview({ loginAttempt: true }).then((loaded) => {
    if (loaded) showDashboard();
    else logout(loginError.textContent || "登录已失效，请重新输入管理员令牌。");
  });
}

async function loadOverview({ loginAttempt = false } = {}) {
  setContentState("loading");
  refreshButton.disabled = true;
  refreshButton.setAttribute("aria-busy", "true");
  loginError.textContent = "";

  try {
    const response = await fetch("/v1/admin/snapshots", {
      headers: { Authorization: `Bearer ${state.token}` },
    });
    if (response.status === 401) {
      if (loginAttempt) loginError.textContent = "管理员令牌无效。";
      else logout("登录已失效，请重新输入管理员令牌。");
      return false;
    }
    if (!response.ok) throw new Error(`服务器返回 ${response.status}`);
    const payload = await response.json();
    if (!payload || !Array.isArray(payload.users) || !Array.isArray(payload.tokenRequests)) {
      throw new Error("服务器返回的数据格式无效");
    }
    state.tokenRequests = payload.tokenRequests;
    state.users = payload.users;
    state.generatedAt = payload.generatedAt;
    updateKpis();
    renderTokenRequests();
    renderUsers();
    freshness.textContent = `更新于 ${formatDateTime(payload.generatedAt)}`;
    return true;
  } catch (error) {
    if (loginAttempt) {
      loginError.textContent = `无法读取云端数据：${error.message}`;
    } else {
      errorMessage.textContent = error.message;
      setContentState("error");
    }
    return false;
  } finally {
    refreshButton.disabled = false;
    refreshButton.removeAttribute("aria-busy");
  }
}

function showDashboard() {
  loginLayer.hidden = true;
  appShell.hidden = false;
}

function logout(message = "") {
  sessionStorage.removeItem("coworkpal-admin-token");
  state.token = "";
  state.users = [];
  state.tokenRequests = [];
  requestList.replaceChildren();
  userList.replaceChildren();
  searchInput.value = "";
  tokenInput.value = "";
  appShell.hidden = true;
  loginLayer.hidden = false;
  loginError.textContent = message;
  tokenInput.focus();
}

function renderTokenRequests() {
  const pending = state.tokenRequests.filter((request) => request.status === "pending");
  requestCount.textContent = `${pending.length} 个待处理`;
  requestEmpty.hidden = pending.length > 0;
  if (pending.length === 0) {
    requestList.replaceChildren();
    return;
  }

  const fragment = document.createDocumentFragment();
  for (const request of pending) {
    const row = element("article", "request-row");
    const identity = element("div", "request-identity");
    identity.append(
      textElement(
        "strong",
        request.kind === "recovery"
          ? `恢复令牌 · ${request.userName || "未命名用户"}`
          : request.userName || "未命名用户",
      ),
      textElement("span", request.userId || request.deviceId || "未知设备"),
    );
    const requestedAt = textElement("time", formatDateTime(request.requestedAt), "request-time");
    const actions = element("div", "request-actions");
    const approve = textElement("button", "发放令牌", "primary-button request-button");
    approve.type = "button";
    approve.addEventListener("click", () => decideTokenRequest(request.requestId, "approve", actions));
    const reject = textElement("button", "拒绝", "secondary-button request-button");
    reject.type = "button";
    reject.addEventListener("click", () => decideTokenRequest(request.requestId, "reject", actions));
    actions.append(approve, reject);
    row.append(identity, requestedAt, actions);
    fragment.append(row);
  }
  requestList.replaceChildren(fragment);
}

async function decideTokenRequest(requestId, decision, actions) {
  const buttons = actions.querySelectorAll("button");
  for (const button of buttons) button.disabled = true;
  try {
    const response = await fetch(`/v1/admin/token-requests/${requestId}/${decision}`, {
      method: "POST",
      headers: { Authorization: `Bearer ${state.token}` },
    });
    if (response.status === 401) {
      logout("登录已失效，请重新输入管理员令牌。");
      return;
    }
    if (!response.ok) {
      const payload = await response.json().catch(() => ({}));
      throw new Error(payload.error || `服务器返回 ${response.status}`);
    }
    await loadOverview();
  } catch (error) {
    window.alert(`处理令牌申请失败：${error.message}`);
    for (const button of buttons) button.disabled = false;
  }
}

function setLoginPending(pending) {
  const submit = loginForm.querySelector("button[type='submit']");
  submit.disabled = pending;
  submit.textContent = pending ? "正在验证" : "进入总览";
}

function setContentState(name) {
  loadingState.hidden = name !== "loading";
  errorState.hidden = name !== "error";
  emptyState.hidden = name !== "empty";
  userList.hidden = name !== "ready";
}

function updateKpis() {
  const ready = state.users.filter((user) => user.backup?.status === "ready");
  const totalOnlineSeconds = ready.reduce(
    (total, user) => total + finiteNumber(user.backup.workshop?.totalOnlineSeconds),
    0,
  );
  const totalNotes = ready.reduce(
    (total, user) => total + finiteNumber(user.backup.counts?.notes),
    0,
  );
  const totalWorkLogDays = ready.reduce(
    (total, user) => total + finiteNumber(user.backup.counts?.workLogDays),
    0,
  );
  const totalAchievements = ready.reduce(
    (total, user) => total + finiteNumber(user.backup.counts?.achievements),
    0,
  );
  document.querySelector("#kpi-users").textContent = formatNumber(state.users.length);
  document.querySelector("#kpi-backed-up").textContent = formatNumber(ready.length);
  document.querySelector("#kpi-online").textContent = formatDurationCompact(totalOnlineSeconds);
  document.querySelector("#kpi-notes").textContent = formatNumber(totalNotes);
  document.querySelector("#kpi-worklogs").textContent = `${formatNumber(totalWorkLogDays)} / ${formatNumber(totalWorkLogDays)}`;
  document.querySelector("#kpi-achievements").textContent = formatNumber(totalAchievements);
}

function renderUsers() {
  const query = searchInput.value.trim().toLocaleLowerCase("zh-CN");
  const users = state.users.filter((user) => {
    const searchable = [
      user.userName,
      user.userId,
      user.backup?.device?.catId,
      user.backup?.appVersion,
      user.backup?.hardware?.cpuName,
      user.backup?.hardware?.gpuName,
    ]
      .filter(Boolean)
      .join(" ")
      .toLocaleLowerCase("zh-CN");
    return searchable.includes(query);
  });

  resultCount.textContent = `${users.length} 位用户`;
  if (users.length === 0) {
    userList.replaceChildren();
    setContentState("empty");
    return;
  }

  const fragment = document.createDocumentFragment();
  for (const user of users) fragment.append(createUserRow(user));
  userList.replaceChildren(fragment);
  setContentState("ready");
}

function createUserRow(user) {
  const row = element("article", "user-row");
  const header = element("div", "user-row-header");
  const summary = element("button", "user-summary");
  summary.type = "button";
  summary.title = "点击查看备份、工坊、数据与设备详情";
  summary.setAttribute("aria-expanded", "false");

  const identity = element("div", "identity");
  const avatar = document.createElement("img");
  avatar.className = "user-avatar";
  avatar.src = "/cocat-avatar.png";
  avatar.alt = "";
  const identityCopy = document.createElement("div");
  identityCopy.append(
    textElement("strong", user.userName || "未命名用户"),
    textElement("span", backupIdentity(user)),
  );
  identity.append(avatar, identityCopy);

  const status = element("div", `status ${statusClass(user.backup)}`);
  status.textContent = statusLabel(user.backup);
  const workshop = summaryMetric(
    user.backup?.status === "ready" ? `Lv.${formatNumber(user.backup.workshop?.level)}` : "-",
    "工坊等级",
    "summary-workshop",
  );
  const focus = summaryMetric(
    user.backup?.status === "ready"
      ? formatDurationCompact(user.backup.workshop?.totalOnlineSeconds)
      : "-",
    "累计在线",
    "summary-focus",
  );
  const chevron = textElement("span", "⌄", "chevron");
  summary.append(identity, status, workshop, focus, chevron);

  const details = element("div", "user-details");
  details.hidden = true;
  details.append(createDetails(user));
  summary.addEventListener("click", () => {
    const expanded = summary.getAttribute("aria-expanded") === "true";
    summary.setAttribute("aria-expanded", String(!expanded));
    details.hidden = expanded;
  });
  header.append(summary);
  if ((user.tokens ?? []).some((token) => token.source === "issued")) {
    const remove = textElement("button", "删除", "secondary-button user-row-delete danger-button");
    remove.type = "button";
    remove.title = `删除用户 ${user.userName}`;
    remove.setAttribute("aria-label", `删除用户 ${user.userName}`);
    remove.addEventListener("click", () => deleteUser(user, remove));
    header.append(remove);
  }
  row.append(header, details);
  return row;
}

function createDetails(user) {
  const backup = user.backup;
  const grid = element("div", "detail-grid");
  grid.append(tokenManagementGroup(user), backupHistoryGroup(user));
  if (!backup || backup.status !== "ready") {
    grid.append(textElement(
      "p",
      backup?.status === "invalid"
        ? "该用户的最新备份无法解析，请检查服务日志或重新上传。"
        : "该用户尚未上传云端备份。",
      "missing-detail",
    ));
    return grid;
  }

  grid.append(
    detailGroup("备份信息", [
      ["备份时间", formatDateTime(backup.storedAt)],
      ["客户端导出", formatTimestamp(backup.exportedAt)],
      ["应用版本", backup.appVersion],
      ["备份大小", formatBytes(backup.sizeBytes)],
      ["修订号", compactRevision(backup.revision)],
    ]),
    detailGroup("工坊状态", [
      ["工坊等级", `Lv.${formatNumber(backup.workshop?.level)}`],
      ["亲密等级", `Lv.${formatNumber(backup.workshop?.affinityLevel)}`],
      ["累计在线", formatDuration(backup.workshop?.totalOnlineSeconds)],
      ["零件 / 今日", `${formatNumber(backup.workshop?.parts)} / +${formatNumber(backup.workshop?.todayParts)}`],
      ["洞察 / 今日", `${formatNumber(backup.workshop?.insight)} / +${formatNumber(backup.workshop?.todayInsight)}`],
    ]),
    detailGroup("云端数据", [
      ["笔记", formatNumber(backup.counts?.notes)],
      ["专注记录", formatNumber(backup.counts?.focusSessions)],
      ["日报", formatNumber(backup.counts?.workLogDays)],
      ["体检", formatNumber(backup.counts?.workLogDays)],
      ["成就", formatNumber(backup.counts?.achievements)],
    ], "backup-data-group"),
    detailGroup("应用设置", [
      ["设备标识", backup.device?.catId],
      ["主题", backup.device?.themeName],
      ["开机启动", yesNo(backup.device?.launchAtStartup)],
      ["低功耗模式", yesNo(backup.device?.lowPowerMode)],
      ["静态 CoCat", yesNo(backup.device?.staticCatMode)],
      ["睡眠模式", yesNo(backup.device?.sleepMode)],
      ["通知", yesNo(backup.device?.notifications)],
      ["硬件监控", yesNo(backup.device?.hardwareMonitor)],
      ["采样间隔", formatMilliseconds(backup.device?.samplingIntervalMs)],
      ["后台采样", formatMilliseconds(backup.device?.backgroundSamplingIntervalMs)],
      ["监控栏模式", backup.device?.monitorBarMode],
      ["监控指标", listValue(backup.device?.visibleMonitorMetrics)],
    ], "detail-group-wide"),
    hardwareGroup(backup.hardware),
    moduleLevels(backup.workshop?.moduleLevels),
  );
  return grid;
}

function tokenManagementGroup(user) {
  const section = element("section", "detail-group token-management");
  section.append(textElement("h3", "用户与令牌"));

  const identity = element("div", "account-identity");
  identity.append(
    textElement("span", "用户唯一 ID"),
    copyableValue(user.userId, "复制用户唯一 ID"),
    textElement("span", `云端历史版本 ${formatNumber(user.historyCount)} 份`, "account-meta"),
  );
  section.append(identity);

  const list = element("div", "managed-token-list");
  for (const token of user.tokens ?? []) {
    const row = element("div", "managed-token");
    const status = textElement(
      "span",
      token.status === "active" ? "使用中" : token.status === "revoked" ? "已停用" : "无法恢复",
      `token-status token-status-${token.status}`,
    );
    const source = textElement(
      "span",
      token.source === "configured" ? "环境配置" : "管理员发放",
      "token-source",
    );
    const heading = element("div", "managed-token-heading");
    heading.append(status, source);

    const field = element("div", "managed-token-field");
    const input = document.createElement("input");
    const hasAccessToken = Boolean(token.accessToken);
    input.type = hasAccessToken ? "password" : "text";
    input.readOnly = true;
    input.value = token.accessToken || "旧令牌未保留；请让客户端同步一次，或重置令牌";
    input.setAttribute("aria-label", `${user.userName} 的访问令牌`);
    field.append(input);

    if (hasAccessToken) {
      const reveal = iconButton("◉", "显示令牌");
      reveal.addEventListener("click", () => {
        const showing = input.type === "text";
        input.type = showing ? "password" : "text";
        reveal.title = showing ? "显示令牌" : "隐藏令牌";
        reveal.setAttribute("aria-label", reveal.title);
      });
      const copy = iconButton("⧉", "复制令牌");
      copy.addEventListener("click", () => copyText(token.accessToken));
      field.append(reveal, copy);
    }

    row.append(heading, field);
    if (token.source === "issued") {
      const actions = element("div", "managed-token-actions");
      const rotate = textElement("button", "重置令牌", "secondary-button request-button");
      rotate.type = "button";
      rotate.addEventListener("click", () => manageToken(user, token, "rotate"));
      const revoke = textElement("button", "停用令牌", "secondary-button request-button danger-button");
      revoke.type = "button";
      revoke.disabled = token.status === "revoked";
      revoke.addEventListener("click", () => manageToken(user, token, "revoke"));
      actions.append(rotate, revoke);
      row.append(actions);
    }
    list.append(row);
  }
  section.append(list);
  return section;
}

function backupHistoryGroup(user) {
  const section = element("section", "detail-group detail-group-wide backup-history");
  const heading = element("div", "backup-history-heading");
  heading.append(
    textElement("h3", "备份记录"),
    textElement("span", `${formatNumber(user.versions?.length)} / 30`, "account-meta"),
  );
  section.append(heading);

  const versions = user.versions ?? [];
  const controls = element("div", "backup-history-controls");
  const select = document.createElement("select");
  select.setAttribute("aria-label", `${user.userName} 的备份记录`);
  for (const version of versions) {
    const option = document.createElement("option");
    option.value = version.revision;
    option.textContent = `${formatDateTime(version.storedAt)} · ${version.appVersion} · ${formatBytes(version.sizeBytes)}`;
    select.append(option);
  }
  if (versions.length === 0) {
    const option = document.createElement("option");
    option.textContent = "暂无备份记录";
    select.append(option);
    select.disabled = true;
  }

  const restore = textElement("button", "推送还原", "secondary-button request-button");
  restore.type = "button";
  restore.disabled = versions.length === 0;
  restore.addEventListener("click", () => restoreSnapshot(user, select, restore));
  controls.append(select, restore);
  section.append(controls);

  if (user.pendingRestore) {
    section.append(
      textElement(
        "p",
        `等待客户端还原 · ${formatDateTime(user.pendingRestore.requestedAt)}`,
        "restore-pending",
      ),
    );
  }
  return section;
}

function copyableValue(value, title) {
  const field = element("div", "copyable-value");
  field.append(textElement("code", value));
  const copy = iconButton("⧉", title);
  copy.addEventListener("click", () => copyText(value));
  field.append(copy);
  return field;
}

function iconButton(symbol, title) {
  const button = textElement("button", symbol, "icon-button compact-icon-button");
  button.type = "button";
  button.title = title;
  button.setAttribute("aria-label", title);
  return button;
}

async function copyText(value) {
  try {
    if (!navigator.clipboard?.writeText) throw new Error("clipboard API unavailable");
    await navigator.clipboard.writeText(value);
    return true;
  } catch {}

  const textarea = document.createElement("textarea");
  textarea.value = value;
  textarea.setAttribute("readonly", "");
  textarea.style.position = "fixed";
  textarea.style.opacity = "0";
  document.body.append(textarea);
  textarea.select();
  const copied = document.execCommand("copy");
  textarea.remove();
  if (!copied) {
    window.prompt("请手动复制以下内容", value);
  }
  return copied;
}

async function manageToken(user, token, action) {
  const message =
    action === "rotate"
      ? "重置后旧令牌立即失效，确定继续吗？"
      : "停用后该设备将无法上传或恢复数据，确定继续吗？";
  if (!window.confirm(message)) return;
  try {
    const response = await fetch(
      `/v1/admin/users/${user.userId}/tokens/${token.tokenId}/${action}`,
      { method: "POST", headers: { Authorization: `Bearer ${state.token}` } },
    );
    if (response.status === 401) {
      logout("登录已失效，请重新输入管理员令牌。");
      return;
    }
    const payload = await response.json().catch(() => ({}));
    if (!response.ok) throw new Error(payload.error || `服务器返回 ${response.status}`);
    if (payload.accessToken) {
      const tokenCheck = await fetch("/v1/snapshot", {
        headers: { Authorization: `Bearer ${payload.accessToken}` },
      });
      if (![200, 404].includes(tokenCheck.status)) {
        throw new Error("新令牌生成后未通过服务器验证，请勿交付给用户");
      }
      const copied = await copyText(payload.accessToken);
      window.alert(
        copied
          ? "新令牌已验证并复制。旧令牌已经失效。"
          : "新令牌已验证，请在令牌栏中手动复制。旧令牌已经失效。",
      );
    }
    await loadOverview();
  } catch (error) {
    window.alert(`令牌操作失败：${error.message}`);
  }
}

async function deleteUser(user, button) {
  const historyCount = formatNumber(user.historyCount);
  if (
    !window.confirm(
      `确定永久删除“${user.userName}”吗？该用户的 ${historyCount} 份备份、全部令牌和待还原任务都会删除，且无法撤销。`,
    )
  ) {
    return;
  }

  button.disabled = true;
  button.textContent = "删除中…";
  try {
    const response = await fetch(`/v1/admin/users/${user.userId}`, {
      method: "DELETE",
      headers: { Authorization: `Bearer ${state.token}` },
    });
    if (response.status === 401) {
      logout("登录已失效，请重新输入管理员令牌。");
      return;
    }
    const payload = await response.json().catch(() => ({}));
    if (!response.ok) throw new Error(payload.error || `服务器返回 ${response.status}`);
    window.alert(`用户“${user.userName}”及其云端数据已删除。`);
    await loadOverview();
  } catch (error) {
    window.alert(`删除用户失败：${error.message}`);
  } finally {
    button.disabled = false;
    button.textContent = "删除";
  }
}

async function restoreSnapshot(user, select, button) {
  const revision = select.value;
  if (!revision) return;
  const selectedLabel = select.selectedOptions[0]?.textContent ?? "所选备份";
  if (!window.confirm(`将 ${selectedLabel} 推送给客户端还原，确定继续吗？`)) return;

  button.disabled = true;
  button.textContent = "推送中…";
  try {
    const response = await fetch(
      `/v1/admin/users/${user.userId}/snapshots/${encodeURIComponent(revision)}/restore`,
      { method: "POST", headers: { Authorization: `Bearer ${state.token}` } },
    );
    if (response.status === 401) {
      logout("登录已失效，请重新输入管理员令牌。");
      return;
    }
    const payload = await response.json().catch(() => ({}));
    if (!response.ok) throw new Error(payload.error || `服务器返回 ${response.status}`);
    window.alert("备份已推送，客户端在线后将在约 30 秒内自动还原。");
    await loadOverview();
  } catch (error) {
    window.alert(`推送还原失败：${error.message}`);
  } finally {
    button.disabled = false;
    button.textContent = "推送还原";
  }
}

function hardwareGroup(hardware = {}) {
  const hasProfile = Boolean(
    hardware?.cpuName ||
      hardware?.gpuName ||
      hardware?.totalMemoryBytes ||
      hardware?.motherboard?.length ||
      hardware?.disks?.length,
  );
  if (!hasProfile) {
    return detailGroup("硬件配置", [
      ["状态", "等待客户端重新上传备份"],
      ["说明", "旧备份不包含 CPU、显卡等硬件信息"],
    ], "detail-group-wide");
  }

  return detailGroup("硬件配置", [
    ["采集时间", formatTimestamp(hardware.capturedAt)],
    ["CPU", hardware.cpuName],
    ["核心 / 线程", formatCoreCounts(hardware)],
    ["显卡", hardware.gpuName || formatDeviceNames(hardware.gpus)],
    ["显存", formatOptionalBytes(hardware.gpuMemoryTotalBytes)],
    ["系统内存", formatOptionalBytes(hardware.totalMemoryBytes)],
    ["主板", formatDeviceNames(hardware.motherboard)],
    ["内存条", formatMemoryModules(hardware.memoryModules)],
    ["磁盘", formatDevicesWithCapacity(hardware.disks)],
    ["显示器", formatDeviceNames(hardware.displays)],
    ["网络适配器", formatDeviceNames(hardware.networkAdapters)],
  ], "detail-group-wide");
}

function detailGroup(title, items, className = "") {
  const group = element("section", `detail-group ${className}`.trim());
  group.append(textElement("h3", title));
  const list = element("dl", "detail-list");
  for (const [label, value] of items) {
    list.append(textElement("dt", label), textElement("dd", safeText(value)));
  }
  group.append(list);
  return group;
}

function moduleLevels(levels = {}) {
  const section = element("section", "detail-group module-levels");
  section.append(textElement("h3", "模块等级"));
  const modules = element("div", "module-grid");
  for (const [key, label] of [
    ["cpu", "CPU"],
    ["gpu", "GPU"],
    ["ram", "RAM"],
    ["network", "网络"],
    ["temperature", "温度"],
    ["disk", "磁盘"],
  ]) {
    const level = levels?.[key] ?? {};
    const item = element("div", "module");
    item.append(
      textElement("strong", label),
      textElement("span", `零件 Lv.${formatNumber(level.parts ?? 1)}`),
      textElement("span", `处理 Lv.${formatNumber(level.process ?? 1)}`),
    );
    modules.append(item);
  }
  section.append(modules);
  return section;
}

function summaryMetric(value, label, className) {
  const metric = element("div", `summary-metric ${className}`);
  metric.append(textElement("strong", value), textElement("span", label));
  return metric;
}

function backupIdentity(user) {
  if (user.backup?.status === "ready") return user.backup.device?.catId || "未知设备";
  if (user.backup?.status === "invalid") return "备份不可读";
  return "等待首次上传";
}

function statusLabel(backup) {
  if (backup?.status === "ready") return "已备份";
  if (backup?.status === "invalid") return "备份异常";
  return "未备份";
}

function statusClass(backup) {
  if (backup?.status === "ready") return "status-ready";
  if (backup?.status === "invalid") return "status-invalid";
  return "status-missing";
}

function element(tag, className) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  return node;
}

function textElement(tag, value, className = "") {
  const node = element(tag, className);
  node.textContent = safeText(value);
  return node;
}

function safeText(value) {
  if (value === null || value === undefined || value === "") return "-";
  return String(value);
}

function finiteNumber(value) {
  const number = Number(value);
  return Number.isFinite(number) ? number : 0;
}

function formatNumber(value) {
  return new Intl.NumberFormat("zh-CN").format(finiteNumber(value));
}

function formatDateTime(value) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "未知时间";
  return new Intl.DateTimeFormat("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  }).format(date);
}

function formatTimestamp(value) {
  const timestamp = finiteNumber(value);
  return timestamp > 0 ? formatDateTime(timestamp) : "未知时间";
}

function formatDurationCompact(value) {
  const seconds = Math.max(0, finiteNumber(value));
  if (seconds < 3600) return `${Math.floor(seconds / 60)} 分钟`;
  return `${new Intl.NumberFormat("zh-CN", { maximumFractionDigits: 1 }).format(seconds / 3600)} 小时`;
}

function formatDuration(value) {
  const seconds = Math.max(0, Math.floor(finiteNumber(value)));
  const days = Math.floor(seconds / 86400);
  const hours = Math.floor((seconds % 86400) / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  if (days > 0) return `${days} 天 ${hours} 小时`;
  if (hours > 0) return `${hours} 小时 ${minutes} 分钟`;
  return `${minutes} 分钟`;
}

function formatBytes(value) {
  const bytes = Math.max(0, finiteNumber(value));
  if (bytes < 1024) return `${bytes} B`;
  const units = ["B", "KB", "MB", "GB", "TB"];
  const unitIndex = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  return `${new Intl.NumberFormat("zh-CN", { maximumFractionDigits: 1 }).format(bytes / 1024 ** unitIndex)} ${units[unitIndex]}`;
}

function formatOptionalBytes(value) {
  return finiteNumber(value) > 0 ? formatBytes(value) : "-";
}

function formatCoreCounts(hardware) {
  const physical = finiteNumber(hardware?.cpuPhysicalCoreCount);
  const logical = finiteNumber(hardware?.cpuLogicalCoreCount);
  if (physical <= 0 && logical <= 0) return "-";
  return `${physical || "-"} 核 / ${logical || "-"} 线程`;
}

function formatDeviceNames(devices) {
  if (!Array.isArray(devices) || devices.length === 0) return "-";
  return devices
    .map((device) => [device?.vendor, device?.name].filter(Boolean).join(" "))
    .filter(Boolean)
    .join("；");
}

function formatDevicesWithCapacity(devices) {
  if (!Array.isArray(devices) || devices.length === 0) return "-";
  return devices
    .map((device) => {
      const capacity = finiteNumber(device?.capacityBytes);
      return `${safeText(device?.name)}${capacity > 0 ? ` (${formatBytes(capacity)})` : ""}`;
    })
    .join("；");
}

function formatMemoryModules(modules) {
  if (!Array.isArray(modules) || modules.length === 0) return "-";
  return modules
    .map((module) => {
      const identity = [module?.manufacturer, module?.partNumber].filter(Boolean).join(" ");
      const capacity = finiteNumber(module?.capacityBytes);
      const speed = finiteNumber(module?.speedMhz);
      return [identity, capacity > 0 ? formatBytes(capacity) : "", speed > 0 ? `${speed} MHz` : ""]
        .filter(Boolean)
        .join(" / ");
    })
    .join("；");
}

function formatMilliseconds(value) {
  const milliseconds = finiteNumber(value);
  if (milliseconds <= 0) return "-";
  if (milliseconds >= 1000) return `${milliseconds / 1000} 秒`;
  return `${milliseconds} 毫秒`;
}

function yesNo(value) {
  return value ? "开启" : "关闭";
}

function listValue(value) {
  return Array.isArray(value) && value.length > 0 ? value.join(" / ") : "无";
}

function compactRevision(value) {
  const revision = safeText(value);
  return revision.length > 20 ? `${revision.slice(0, 18)}…` : revision;
}
