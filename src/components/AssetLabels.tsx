import type { Category, Tag } from "../types";

type Props = { tags: Tag[]; categories: Category[]; tagIds: string[]; categoryIds: string[]; setTagIds: (ids: string[]) => void; setCategoryIds: (ids: string[]) => void };

export function AssetLabels({ tags, categories, tagIds, categoryIds, setTagIds, setCategoryIds }: Props) {
  const toggle = (ids: string[], id: string, set: (ids: string[]) => void) => set(ids.includes(id) ? ids.filter((value) => value !== id) : [...ids, id]);
  return <div className="asset-labels"><section><b>カテゴリー</b><div>{categories.map((category) => <label className="check" key={category.id}><input type="radio" name="asset-category" checked={categoryIds.includes(category.id)} onClick={() => setCategoryIds(categoryIds.includes(category.id) ? [] : [category.id])} readOnly />{category.name}</label>)}</div></section><section><b>タグ</b><div>{tags.map((tag) => <label className="check" key={tag.id}><input type="checkbox" checked={tagIds.includes(tag.id)} onChange={() => toggle(tagIds, tag.id, setTagIds)} />{tag.name}</label>)}</div></section></div>;
}
