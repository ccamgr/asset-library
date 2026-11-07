import { useEffect } from "react";

type Category = { id: string; name: string };
type Options = {
  enabled: boolean;
  categories: Category[];
  selectedId: string | null;
  onSelect: (id: string | null) => void;
};

/** Renders the category filter above the existing tag filter in the sidebar. */
export function useCategorySidebar({ enabled, categories, selectedId, onSelect }: Options) {
  useEffect(() => {
    if (!enabled) return;
    const sidebar = document.querySelector<HTMLElement>(".sidebar");
    const tagSection = sidebar?.querySelector<HTMLElement>(".side-section");
    if (!sidebar || !tagSection) return;
    const section = document.createElement("div");
    section.className = "side-section category-filter-section";
    const title = document.createElement("div");
    title.className = "side-title";
    const heading = document.createElement("b");
    heading.textContent = "カテゴリー";
    title.append(heading);
    section.append(title);

    const addFilter = (label: string, id: string | null) => {
      const row = document.createElement("div");
      row.className = "category-filter-row";
      if (id) row.dataset.categoryId = id;
      const button = document.createElement("button");
      button.type = "button";
      button.className = selectedId === id ? "filter active" : "filter";
      button.textContent = label;
      button.addEventListener("click", () => onSelect(id));
      row.append(button);
      section.append(row);
    };
    addFilter("すべて", null);
    categories.forEach((category) => addFilter(category.name, category.id));
    sidebar.insertBefore(section, tagSection);
    return () => section.remove();
  }, [categories, enabled, onSelect, selectedId]);
}
