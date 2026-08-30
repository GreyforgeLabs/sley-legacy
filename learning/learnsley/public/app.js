const state = {
  trials: [],
  progress: null,
  currentTrialId: null,
  hintIndex: -1,
  lastGraph: null,
  csrfToken: null
};

const els = {
  trialList: document.querySelector("#trialList"),
  campaignCount: document.querySelector("#campaignCount"),
  trialMeta: document.querySelector("#trialMeta"),
  trialTitle: document.querySelector("#trialTitle"),
  concepts: document.querySelector("#concepts"),
  instructions: document.querySelector("#instructions"),
  task: document.querySelector("#task"),
  misconception: document.querySelector("#misconception"),
  editor: document.querySelector("#editor"),
  output: document.querySelector("#output"),
  diagnostics: document.querySelector("#diagnostics"),
  graph: document.querySelector("#graph"),
  hintText: document.querySelector("#hintText"),
  runButton: document.querySelector("#runButton"),
  sealButton: document.querySelector("#sealButton"),
  formatButton: document.querySelector("#formatButton"),
  graphButton: document.querySelector("#graphButton"),
  hintButton: document.querySelector("#hintButton"),
  resetButton: document.querySelector("#resetButton"),
  levelBadge: document.querySelector("#levelBadge"),
  xpBadge: document.querySelector("#xpBadge"),
  chainBadge: document.querySelector("#chainBadge"),
  armorBadge: document.querySelector("#armorBadge")
};

function currentTrial() {
  return state.trials.find((trial) => trial.id === state.currentTrialId);
}

function codeKey(trialId) {
  return `learnsley.code.${trialId}`;
}

function loadSavedCode(trial) {
  return localStorage.getItem(codeKey(trial.id)) || trial.starter || "";
}

function saveCurrentCode() {
  const trial = currentTrial();
  if (!trial) {
    return;
  }
  localStorage.setItem(codeKey(trial.id), els.editor.value);
}

async function api(path, options = {}) {
  const headers = { ...(options.headers || {}) };
  if (options.body !== undefined) {
    headers["content-type"] = "application/json";
  }
  if (options.method && options.method !== "GET") {
    headers["x-learnsley-csrf"] = state.csrfToken;
  }
  const response = await fetch(path, {
    ...options,
    headers
  });
  const payload = await response.json();
  if (!response.ok) {
    throw new Error(payload.error || `request failed: ${response.status}`);
  }
  return payload;
}

function pretty(value) {
  if (value === null || value === undefined) {
    return "";
  }
  if (typeof value === "string") {
    return value;
  }
  return JSON.stringify(value, null, 2);
}

function setOutput(text, ok = null) {
  els.output.className = ok === true ? "success" : ok === false ? "failure" : "";
  els.output.textContent = text || "";
}

function setDiagnostics(text, ok = null) {
  els.diagnostics.className = ok === true ? "success" : ok === false ? "failure" : "";
  els.diagnostics.textContent = text || "";
}

function setGraph(value) {
  els.graph.textContent = pretty(value);
}

function renderProgress() {
  const progress = state.progress || {};
  els.levelBadge.textContent = `Level ${progress.level || 1}`;
  els.xpBadge.textContent = `${progress.xp || 0} XP`;
  els.chainBadge.textContent = `Chain ${progress.cleanSealChain || 0}`;
  els.armorBadge.textContent = `Armor ${progress.proofArmor ?? 0}`;

  const completed = progress.completed || {};
  const done = state.trials.filter((trial) => completed[trial.id]).length;
  els.campaignCount.textContent = `${done}/${state.trials.length}`;
}

function renderTrialList() {
  const completed = state.progress?.completed || {};
  els.trialList.innerHTML = "";
  for (const trial of state.trials) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = [
      "trial-button",
      trial.id === state.currentTrialId ? "active" : "",
      completed[trial.id] ? "done" : ""
    ].filter(Boolean).join(" ");
    button.innerHTML = `
      <span class="trial-number">${trial.order}</span>
      <span>
        <span class="trial-title">${trial.title}</span>
        <span class="trial-subtitle">${trial.campaign}</span>
      </span>
    `;
    button.addEventListener("click", () => selectTrial(trial.id));
    els.trialList.append(button);
  }
}

function renderTrial() {
  const trial = currentTrial();
  if (!trial) {
    return;
  }

  els.trialMeta.textContent = `${trial.campaign} / ${trial.arc} / ${trial.estimatedMinutes} min`;
  els.trialTitle.textContent = trial.title;
  els.instructions.textContent = trial.instructions;
  els.task.textContent = trial.task;
  els.misconception.textContent = trial.misconception;
  els.concepts.innerHTML = "";
  for (const concept of trial.concepts || []) {
    const badge = document.createElement("span");
    badge.textContent = concept;
    els.concepts.append(badge);
  }

  els.editor.value = loadSavedCode(trial);
  els.editor.setAttribute("aria-label", trial.mode === "explanation" ? "Answer editor" : "Sley editor");
  els.formatButton.disabled = trial.mode !== "code";
  els.graphButton.disabled = trial.mode !== "code";
  state.hintIndex = -1;
  els.hintText.textContent = "No hint used.";
  setOutput("");
  setDiagnostics("");
  setGraph("");
  renderTrialList();
  renderProgress();
}

function selectTrial(trialId) {
  saveCurrentCode();
  state.currentTrialId = trialId;
  renderTrial();
}

function renderRun(payload) {
  const run = payload.run;
  if (payload.progress) {
    state.progress = payload.progress;
  }
  renderProgress();
  renderTrialList();

  if (run.phase === "explanation") {
    setOutput("Explanation drafted. Press Seal when it names the rule.", true);
    setDiagnostics("");
    setGraph("");
    return;
  }

  if (run.ok) {
    setOutput(pretty(run.output), true);
    setDiagnostics(pretty(run.checkJson), true);
    setGraph(run.graphJson || run.graphRaw || "");
  } else {
    const detail = run.checkJson || run.run?.stderr || run.check?.stderr || run.run?.stdout || run.check?.stdout;
    setOutput(run.output ? pretty(run.output) : "Run did not complete.", false);
    setDiagnostics(pretty(detail), false);
    setGraph("");
  }
}

async function runCurrent() {
  const trial = currentTrial();
  saveCurrentCode();
  setOutput("Running...");
  setDiagnostics("");
  const payload = await api("/api/run", {
    method: "POST",
    body: JSON.stringify({ trialId: trial.id, code: els.editor.value })
  });
  renderRun(payload);
}

async function sealCurrent() {
  const trial = currentTrial();
  saveCurrentCode();
  setOutput("Sealing...");
  setDiagnostics("");
  const payload = await api("/api/seal", {
    method: "POST",
    body: JSON.stringify({ trialId: trial.id, code: els.editor.value })
  });
  state.progress = payload.progress;
  renderProgress();
  renderTrialList();
  renderRun({ run: payload.seal.run, progress: payload.progress });
  const awards = payload.awards?.length ? `\n${payload.awards.join("\n")}` : "";
  const detail = payload.seal.detail ? `\n${pretty(payload.seal.detail)}` : "";
  setDiagnostics(`${payload.seal.reason}${awards}${detail}`, payload.ok);
}

async function formatCurrent() {
  const trial = currentTrial();
  saveCurrentCode();
  const payload = await api("/api/format", {
    method: "POST",
    body: JSON.stringify({ trialId: trial.id, code: els.editor.value })
  });
  if (payload.ok) {
    els.editor.value = payload.formatted;
    saveCurrentCode();
    setDiagnostics("Formatted.", true);
  } else {
    setDiagnostics(payload.stderr || "Format failed.", false);
  }
}

async function graphCurrent() {
  const trial = currentTrial();
  saveCurrentCode();
  const payload = await api("/api/graph", {
    method: "POST",
    body: JSON.stringify({ trialId: trial.id, code: els.editor.value })
  });
  if (payload.ok) {
    setGraph(payload.graph);
    setDiagnostics("Graph loaded.", true);
  } else {
    setGraph(payload.stdout || payload.stderr || "Graph failed.");
    setDiagnostics(payload.stderr || "Graph failed.", false);
  }
}

function showHint() {
  const trial = currentTrial();
  const hints = trial.hints || [];
  if (hints.length === 0) {
    els.hintText.textContent = "No hint available.";
    return;
  }
  state.hintIndex = Math.min(state.hintIndex + 1, hints.length - 1);
  els.hintText.textContent = hints[state.hintIndex];
}

function resetCurrent() {
  const trial = currentTrial();
  localStorage.removeItem(codeKey(trial.id));
  renderTrial();
}

async function boot() {
  const session = await api("/api/session");
  state.csrfToken = session.csrfToken;
  const [trialsPayload, progressPayload] = await Promise.all([
    api("/api/trials"),
    api("/api/progress")
  ]);
  state.trials = trialsPayload.trials;
  state.progress = progressPayload.progress;
  state.currentTrialId = state.trials[0]?.id || null;
  renderTrialList();
  renderProgress();
  renderTrial();
}

els.editor.addEventListener("input", saveCurrentCode);
els.runButton.addEventListener("click", () => runCurrent().catch((error) => setDiagnostics(error.message, false)));
els.sealButton.addEventListener("click", () => sealCurrent().catch((error) => setDiagnostics(error.message, false)));
els.formatButton.addEventListener("click", () => formatCurrent().catch((error) => setDiagnostics(error.message, false)));
els.graphButton.addEventListener("click", () => graphCurrent().catch((error) => setDiagnostics(error.message, false)));
els.hintButton.addEventListener("click", showHint);
els.resetButton.addEventListener("click", resetCurrent);

boot().catch((error) => {
  setDiagnostics(error.message, false);
});
