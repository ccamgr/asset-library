export function confirmDialog(title: string, message: string, confirmLabel = "削除") {
  return new Promise<boolean>((resolve) => {
    const backdrop = document.createElement("div");
    backdrop.className = "text-dialog-backdrop";
    const dialog = document.createElement("section");
    dialog.className = "text-dialog confirm-dialog";
    dialog.setAttribute("role", "alertdialog");
    dialog.setAttribute("aria-modal", "true");
    const heading = document.createElement("h2"); heading.textContent = title;
    const body = document.createElement("p"); body.textContent = message;
    const actions = document.createElement("div"); actions.className = "text-dialog__actions";
    const cancel = document.createElement("button"); cancel.className = "secondary"; cancel.textContent = "キャンセル";
    const confirm = document.createElement("button"); confirm.className = "primary"; confirm.textContent = confirmLabel;
    const close = (result: boolean) => { backdrop.remove(); resolve(result); };
    cancel.addEventListener("click", () => close(false)); confirm.addEventListener("click", () => close(true));
    backdrop.addEventListener("pointerdown", (event) => { if (event.target === backdrop) close(false); });
    dialog.addEventListener("keydown", (event) => { if (event.key === "Escape") close(false); });
    actions.append(cancel, confirm); dialog.append(heading, body, actions); backdrop.append(dialog); document.body.append(backdrop); cancel.focus();
  });
}
