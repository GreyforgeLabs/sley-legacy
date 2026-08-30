const output = document.querySelector("#output");
const targetInput = document.querySelector("#target");
const statusNode = document.querySelector("#status");
let csrfToken = "";

async function api(path, options = {}) {
  const response = await fetch(path, { credentials: "same-origin", ...options });
  const payload = await response.json();
  if (!response.ok) throw new Error(payload.error?.message || `request failed (${response.status})`);
  return payload;
}

async function runCommand(command) {
  const target = (targetInput.value || ".").trim() || ".";
  output.textContent = `Running ${command}...`;
  statusNode.textContent = "status: running";
  try {
    const json = await api("/api/run", {
      method: "POST",
      headers: {
        "content-type": "application/json",
        "x-sley-workbench-csrf": csrfToken,
      },
      body: JSON.stringify({ command, target }),
    });
    statusNode.textContent = `status: ${json.ok ? "ok" : "fail"} (${json.command})`;
    output.textContent = JSON.stringify(json, null, 2);
  } catch (error) {
    statusNode.textContent = "status: error";
    output.textContent = error.message;
  }
}

async function boot() {
  const session = await api("/api/session");
  csrfToken = session.csrfToken;
  statusNode.textContent = `status: idle | v${session.version}`;
  for (const button of document.querySelectorAll("button[data-command]")) {
    button.addEventListener("click", () => runCommand(button.dataset.command));
  }
}

boot().catch((error) => {
  statusNode.textContent = "status: unavailable";
  output.textContent = error.message;
});
