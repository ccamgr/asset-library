import type { Category, Tag } from "../types";

type Props = { tags: Tag[]; categories: Category[]; tagIds: string[]; categoryIds: string[]; setTagIds: (ids: string[]) => void; setCategoryIds: (ids: string[]) => void };

export function ImportLabels({ tags, categories, tagIds, categoryIds, setTagIds, setCategoryIds }: Props) {
  const toggle = (ids: string[], id: string, set: (ids: string[]) => void) => set(ids.includes(id) ? ids.filter((value) => value !== id) : [...ids, id]);
  return <div className="import-labels"><span>まとめて付与:</span>{tags.map((tag) => <label className="check" key={tag.id}><input type="checkbox" checked={tagIds.includes(tag.id)} onChange={() => toggle(tagIds, tag.id, setTagIds)} />{tag.name}</label>)}{categories.map((category) => <label className="check" key={category.id}><input type="checkbox" checked={categoryIds.includes(category.id)} onChange={() => toggle(categoryIds, category.id, setCategoryIds)} />{category.name}</label>)}</div>;
}
