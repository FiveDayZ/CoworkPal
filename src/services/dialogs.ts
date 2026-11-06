export function confirmAction(message: string): Promise<boolean> {
  return new Promise((resolve) => {
    const dialog = document.createElement("dialog");
    dialog.className = "cwp-confirm-dialog";
    dialog.setAttribute("aria-label", "确认操作");
    const title = document.createElement("h2");
    title.textContent = "确认操作";
    const content = document.createElement("p");
    content.textContent = message;
    const actions = document.createElement("div");
    const cancel = document.createElement("button");
    cancel.type = "button";
    cancel.className = "cwp-custom-btn";
    cancel.textContent = "取消";
    cancel.autofocus = true;
    cancel.onclick = () => dialog.close("cancel");
    const confirm = document.createElement("button");
    confirm.type = "button";
    confirm.className = "cwp-custom-btn";
    confirm.textContent = "确认";
    confirm.onclick = () => dialog.close("confirm");
    actions.append(cancel, confirm);
    dialog.append(title, content, actions);
    dialog.addEventListener("close", () => {
      dialog.remove();
      resolve(dialog.returnValue === "confirm");
    }, { once: true });
    document.body.append(dialog);
    try {
      dialog.showModal();
    } catch (error) {
      dialog.remove();
      throw error;
    }
  });
}
