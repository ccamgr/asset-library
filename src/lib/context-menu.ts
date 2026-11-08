import { useEffect, type ReactNode } from "react";
import { createRoot, type Root } from "react-dom/client";

export type ContextMenuItem = {
  label: string;
  icon?: ReactNode | null;
  destructive?: boolean;
  run: () => void | Promise<void>;
};

type ContextMenuResolver = (event: MouseEvent) => ContextMenuItem[] | null;

/**
 * Disables the browser context menu across the application and renders the
 * supplied actions as a small, keyboard-operable app menu.
 */
export function useContextMenu(resolve: ContextMenuResolver) {
  useEffect(() => {
    let menu: HTMLDivElement | null = null;
    const iconRoots: Root[] = [];

    const close = () => {
      menu?.remove();
      iconRoots.splice(0).forEach((root) => root.unmount());
      menu = null;
    };

    const open = (event: MouseEvent, items: ContextMenuItem[]) => {
      close();
      menu = document.createElement("div");
      menu.className = "context-menu";
      menu.setAttribute("role", "menu");
      menu.setAttribute("aria-label", "操作メニュー");
      menu.style.left = `${Math.min(event.clientX, window.innerWidth - 224)}px`;
      menu.style.top = `${Math.min(event.clientY, window.innerHeight - 72 * items.length)}px`;

      for (const item of items) {
        const button = document.createElement("button");
        button.type = "button";
        button.setAttribute("role", "menuitem");
        button.className = item.destructive
          ? "context-menu__item destructive"
          : "context-menu__item";
        if (item.icon) {
          const holder = document.createElement("span");
          holder.className = "context-menu__icon";
          button.append(holder);
          const root = createRoot(holder);
          root.render(item.icon);
          iconRoots.push(root);
        }
        button.append(document.createTextNode(item.label));
        button.addEventListener("click", () => {
          close();
          void item.run();
        });
        menu.append(button);
      }
      document.body.append(menu);
      menu.querySelector<HTMLButtonElement>("button")?.focus();
    };

    const onContextMenu = (event: MouseEvent) => {
      event.preventDefault();
      const items = resolve(event);
      if (items?.length) open(event, items);
      else close();
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") close();
    };
    const onPointerDown = (event: PointerEvent) => {
      if (!menu?.contains(event.target as Node)) close();
    };
    const onDragStart = (event: DragEvent) => {
      const target = event.target;
      if (
        !(
          target instanceof HTMLInputElement ||
          target instanceof HTMLTextAreaElement
        )
      ) {
        event.preventDefault();
      }
    };

    document.addEventListener("contextmenu", onContextMenu);
    document.addEventListener("pointerdown", onPointerDown);
    document.addEventListener("dragstart", onDragStart);
    window.addEventListener("blur", close);
    window.addEventListener("resize", close);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      close();
      document.removeEventListener("contextmenu", onContextMenu);
      document.removeEventListener("pointerdown", onPointerDown);
      document.removeEventListener("dragstart", onDragStart);
      window.removeEventListener("blur", close);
      window.removeEventListener("resize", close);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, [resolve]);
}
