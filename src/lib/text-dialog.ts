export type TextDialogOptions = {
  title: string;
  label: string;
  initialValue: string;
  confirmLabel: string;
  onSubmit: (value: string) => Promise<void>;
};

/** A compact modal for a single text value, shared by context-menu actions. */
export function openTextDialog(options: TextDialogOptions) {
  const backdrop = document.createElement("div");
  backdrop.className = "text-dialog-backdrop";
  const dialog = document.createElement("section");
  dialog.className = "text-dialog";
  dialog.setAttribute("role", "dialog");
  dialog.setAttribute("aria-modal", "true");
  dialog.setAttribute("aria-label", options.title);

  const title = document.createElement("h2");
  title.textContent = options.title;
  const form = document.createElement("form");
  const label = document.createElement("label");
  label.textContent = options.label;
  const input = document.createElement("input");
  input.value = options.initialValue;
  input.required = true;
  input.maxLength = 100;
  label.append(input);
  const error = document.createElement("p");
  error.className = "text-dialog__error";
  const actions = document.createElement("div");
  actions.className = "text-dialog__actions";
  const cancel = document.createElement("button");
  cancel.type = "button";
  cancel.className = "secondary";
  cancel.textContent = "キャンセル";
  const submit = document.createElement("button");
  submit.type = "submit";
  submit.className = "primary";
  submit.textContent = options.confirmLabel;
  actions.append(cancel, submit);
  form.append(label, error, actions);
  dialog.append(title, form);
  backdrop.append(dialog);

  const close = () => backdrop.remove();
  cancel.addEventListener("click", close);
  backdrop.addEventListener("pointerdown", (event) => {
    if (event.target === backdrop) close();
  });
  dialog.addEventListener("keydown", (event) => {
    if (event.key === "Escape") close();
  });
  form.addEventListener("submit", async (event) => {
    event.preventDefault();
    const value = input.value.trim();
    if (!value) return;
    submit.disabled = true;
    cancel.disabled = true;
    error.textContent = "";
    try {
      await options.onSubmit(value);
      close();
    } catch (reason) {
      error.textContent = String(reason);
      submit.disabled = false;
      cancel.disabled = false;
      input.focus();
    }
  });

  document.body.append(backdrop);
  input.focus();
  input.select();
}
