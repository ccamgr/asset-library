import { Plus, Settings } from "lucide-react";
import type { Category, Tag } from "../types";

type Props = {
  tags: Tag[]; categories: Category[];
  categoryFilter: string | null; tagFilterIds: string[]; tagSearch: string; busy: boolean;
  onOpenAssetDialog(): void; onOpenSettings(): void;
  onAddEntity(kind: "tag" | "category"): void; onCategoryFilter(id: string | null): void;
  onTagFilterIds(ids: string[]): void; onTagSearch(value: string): void;
};

export function AssetSidebar(props: Props) {
  const visibleTags = props.tags.filter((tag) => tag.name.toLocaleLowerCase().includes(props.tagSearch.toLocaleLowerCase()));
  const toggleTag = (id: string) => props.onTagFilterIds(props.tagFilterIds.includes(id) ? props.tagFilterIds.filter((value) => value !== id) : [...props.tagFilterIds, id]);
  return <aside className="sidebar">
    <p className="eyebrow">ASSETLIBRARY</p>
    <button className="primary full" onClick={props.onOpenAssetDialog}><Plus aria-hidden size={18} /> 追加</button>
    <div className="side-section category-filter-section"><div className="side-title"><b>カテゴリー</b><button className="icon-button" aria-label="カテゴリーを追加" onClick={() => props.onAddEntity("category")}><Plus aria-hidden size={18} /></button></div><div className="category-filter-list"><button className={!props.categoryFilter ? "filter active" : "filter"} onClick={() => props.onCategoryFilter(null)}>すべて</button>{props.categories.map((category) => <div className="category-filter-row" data-category-id={category.id} key={category.id}><button className={props.categoryFilter === category.id ? "filter active" : "filter"} onClick={() => props.onCategoryFilter(category.id)}>{category.name}</button></div>)}</div></div>
    <div className="side-section"><div className="side-title"><b>タグ</b><div className="side-actions"><button className="secondary compact" disabled={!props.tagFilterIds.length} onClick={() => props.onTagFilterIds([])}>全解除</button><button className="icon-button" aria-label="タグを追加" onClick={() => props.onAddEntity("tag")}><Plus aria-hidden size={18} /></button></div></div><input className="tag-filter-search" type="search" placeholder="タグを検索" value={props.tagSearch} onChange={(event) => props.onTagSearch(event.target.value)} /><div className="tag-filter-list">{visibleTags.map((tag) => <label className="tag-filter-row check" data-tag-id={tag.id} key={tag.id}><input type="checkbox" checked={props.tagFilterIds.includes(tag.id)} onChange={() => toggleTag(tag.id)} />{tag.name}</label>)}</div></div>
    <button className="icon-button settings-launcher" aria-label="設定を開く" title="設定" onClick={props.onOpenSettings}><Settings aria-hidden size={28} /></button>
  </aside>;
}
