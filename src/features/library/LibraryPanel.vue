<script setup lang="ts">
import HoverScrollText from "../../components/HoverScrollText.vue";
import { computed, ref, onMounted, onUnmounted, watch } from "vue";
import {
  ArrowLeft,
  BookOpen,
  ChartNoAxesColumn,
  ChevronDown,
  ChevronRight,
  Cpu,
  Download,
  Settings,
  Folder,
  LoaderCircle,
  Pin,
  PinOff,
  Plus,
  Search,
  Star,
  Trash2,
} from "lucide-vue-next";
import { NButton, NInput } from "naive-ui";
import { desktop, type Workspace } from "../../lib/tauri";

const props = defineProps<{
  data: Workspace;
  sendingDraftIds: string[];
  updateAvailable?: boolean;
  activeUtility?: "projects" | "model" | "settings" | "templates" | "usage";
}>();
const todayCost = ref<string>("—");
let refreshTimer: ReturnType<typeof setInterval>;
let disposed = false;
let request = 0;
async function refreshTodayCost() {
  const current = ++request;
  const since = new Date();
  since.setHours(0, 0, 0, 0);
  try {
    const result = await desktop.usageStats({
      since: Math.floor(since.getTime() / 1000),
      page: 0,
      group: "logs",
      filter: null,
    });
    if (!disposed && current === request)
      todayCost.value =
        result.summary.cost === null
          ? result.summary.count
            ? "—"
            : "¥0.0000"
          : new Intl.NumberFormat("zh-CN", {
              style: "currency",
              currency: "CNY",
              minimumFractionDigits: 4,
              maximumFractionDigits: 4,
            }).format(result.summary.cost);
  } catch {
    if (!disposed && current === request) todayCost.value = "—";
  }
}
onMounted(() => {
  void refreshTodayCost();
  refreshTimer = setInterval(() => void refreshTodayCost(), 60000);
  window.addEventListener("usage-updated", refreshTodayCost);
  window.addEventListener("focus", refreshTodayCost);
});
onUnmounted(() => {
  disposed = true;
  clearInterval(refreshTimer);
  window.removeEventListener("usage-updated", refreshTodayCost);
  window.removeEventListener("focus", refreshTodayCost);
});
watch(
  () => props.activeUtility,
  () => void refreshTodayCost(),
);
const emit = defineEmits<{
  create: [];
  browse: [];
  update: [];
  deleteDraft: [id: string];
  togglePin: [id: string];
  open: [id: string];
  draftContext: [id: string, event: MouseEvent];
  utilities: [
    section: "projects" | "model" | "settings" | "templates" | "usage",
  ];
}>();
const recentExpanded = defineModel<boolean>("recentExpanded", {
  required: true,
});
const pinnedExpanded = defineModel<boolean>("pinnedExpanded", {
  required: true,
});
const listMode = defineModel<"recent" | "favorites">("listMode", {
  required: true,
});
const query = ref("");
const favorites = computed(() => listMode.value === "favorites");

function toggleFavorites() {
  listMode.value = favorites.value ? "recent" : "favorites";
  emit("browse");
}
function returnToRecent() {
  listMode.value = "recent";
  emit("browse");
}

const orderedDrafts = computed(() =>
  [...props.data.drafts].sort(
    (a, b) => (b.updated ?? b.opened) - (a.updated ?? a.opened),
  ),
);
const projectNames = computed(
  () =>
    new Map(props.data.projects.map((project) => [project.id, project.name])),
);
const draftSections = computed(() => {
  const all = orderedDrafts.value;
  const term = query.value.trim().toLowerCase();
  if (term) {
    return [
      {
        id: "search",
        label: "全局搜索结果",
        items: all.filter((item) =>
          [item.title, item.source, item.result].some((text) =>
            text.toLowerCase().includes(term),
          ),
        ),
      },
    ];
  }

  const pinned = all.filter((item) => item.pinned);
  const unpinned = all.filter((item) => !item.pinned);
  const list = favorites.value
    ? {
        id: "favorites",
        label: "收藏",
        items: all.filter((item) => item.favorite),
      }
    : { id: "recent", label: "最近", items: unpinned };
  return [
    ...(pinned.length ? [{ id: "pinned", label: "置顶", items: pinned }] : []),
    list,
  ];
});
</script>

<template>
  <aside class="library" aria-label="草稿资料库">
    <n-button block @click="emit('create')"
      ><template #icon><Plus aria-hidden="true" /></template>新建草稿</n-button
    >
    <n-input v-model:value="query" aria-label="搜索草稿" placeholder="搜索草稿">
      <template #prefix><Search :size="15" aria-hidden="true" /></template>
    </n-input>
    <div class="library-scroll">
      <nav class="library-links" aria-label="资料库">
        <button
          :class="{ selected: !activeUtility && favorites }"
          :aria-pressed="!activeUtility && favorites"
          @click="toggleFavorites"
        >
          <Star />收藏
        </button>
      </nav>
      <template v-for="section in draftSections" :key="section.id">
        <div
          class="library-label"
          :class="{
            'library-label-hover':
              section.id === 'recent' || section.id === 'pinned',
          }"
        >
          <div class="library-label-actions">
            <span class="library-section-title" :title="section.label">{{
              section.label
            }}</span>
            <n-button
              v-if="section.id === 'recent' || section.id === 'pinned'"
              class="recent-toggle"
              quaternary
              size="tiny"
              :aria-expanded="
                section.id === 'pinned' ? pinnedExpanded : recentExpanded
              "
              :aria-label="
                (section.id === 'pinned' ? pinnedExpanded : recentExpanded)
                  ? '收起' + section.label
                  : '展开' + section.label
              "
              :title="
                (section.id === 'pinned' ? pinnedExpanded : recentExpanded)
                  ? '收起' + section.label
                  : '展开' + section.label
              "
              @click="
                section.id === 'pinned'
                  ? (pinnedExpanded = !pinnedExpanded)
                  : (recentExpanded = !recentExpanded)
              "
            >
              <template #icon>
                <ChevronDown
                  v-if="
                    section.id === 'pinned' ? pinnedExpanded : recentExpanded
                  "
                  aria-hidden="true"
                />
                <ChevronRight v-else aria-hidden="true" />
              </template>
            </n-button>
            <n-button
              v-if="section.id === 'favorites'"
              class="recent-toggle"
              quaternary
              size="tiny"
              aria-label="返回最近"
              title="返回最近"
              @click="returnToRecent"
            >
              <template #icon><ArrowLeft aria-hidden="true" /></template>
            </n-button>
          </div>
          <span class="library-label-end">{{ section.items.length }}</span>
        </div>
        <div
          v-if="
            section.id === 'pinned'
              ? pinnedExpanded
              : section.id !== 'recent' || recentExpanded
          "
          class="draft-list"
        >
          <div
            v-for="item in section.items"
            :key="item.id"
            class="library-row draft-row"
            @contextmenu.prevent.stop="emit('draftContext', item.id, $event)"
          >
            <button
              type="button"
              :class="{
                selected: !activeUtility && item.id === data.activeDraft,
              }"
              :aria-current="
                !activeUtility && item.id === data.activeDraft
                  ? 'true'
                  : undefined
              "
              :aria-busy="sendingDraftIds.includes(item.id)"
              @click="emit('open', item.id)"
            >
              <span class="draft-row-label">
                <hover-scroll-text :text="item.title || '未命名草稿'" />
                <small
                  v-if="
                    section.id === 'pinned' ||
                    section.id === 'favorites' ||
                    section.id === 'search'
                  "
                >
                  {{
                    item.projectId
                      ? projectNames.get(item.projectId)
                      : "未分配项目"
                  }}
                </small>
              </span>
              <span
                v-if="sendingDraftIds.includes(item.id)"
                class="draft-sending"
                role="status"
                aria-label="正在发送或处理"
                title="正在发送或处理"
              >
                <LoaderCircle :size="16" aria-hidden="true" />
              </span>
            </button>
            <div class="draft-row-actions">
              <n-button
                quaternary
                size="tiny"
                :title="item.pinned ? '取消置顶' : '置顶草稿'"
                :aria-label="
                  (item.pinned ? '取消置顶 ' : '置顶草稿 ') +
                  (item.title || '未命名草稿')
                "
                :aria-pressed="!!item.pinned"
                @click="emit('togglePin', item.id)"
              >
                <template #icon>
                  <PinOff v-if="item.pinned" aria-hidden="true" />
                  <Pin v-else aria-hidden="true" />
                </template>
              </n-button>
              <n-button
                quaternary
                size="tiny"
                title="删除草稿"
                :aria-label="'删除草稿 ' + (item.title || '未命名草稿')"
                @click="emit('deleteDraft', item.id)"
              >
                <template #icon><Trash2 aria-hidden="true" /></template>
              </n-button>
            </div>
          </div>
          <p
            v-if="
              !section.items.length &&
              (section.id !== 'recent' || !orderedDrafts.length)
            "
            class="library-empty"
          >
            暂无匹配草稿
          </p>
        </div>
      </template>
    </div>
    <nav class="library-links library-utilities" aria-label="工具菜单">
      <button
        :class="{ selected: activeUtility === 'projects' }"
        :aria-current="activeUtility === 'projects' ? 'page' : undefined"
        @click="emit('utilities', 'projects')"
      >
        <Folder aria-hidden="true" /><span class="utility-label">项目</span>
      </button>
      <button
        :class="{ selected: activeUtility === 'templates' }"
        :aria-current="activeUtility === 'templates' ? 'page' : undefined"
        @click="emit('utilities', 'templates')"
      >
        <BookOpen aria-hidden="true" /><span class="utility-label">模板</span>
      </button>
      <button
        :class="{ selected: activeUtility === 'usage' }"
        :aria-current="activeUtility === 'usage' ? 'page' : undefined"
        @click="emit('utilities', 'usage')"
      >
        <ChartNoAxesColumn aria-hidden="true" /><span class="utility-label"
          >用量</span
        >
        <small class="today-cost" :title="'今日预估成本：' + todayCost"
          >今日 {{ todayCost }}</small
        >
      </button>
      <button
        v-if="updateAvailable"
        class="library-update-entry"
        aria-label="有可用更新，前往关于页面"
        @click="emit('update')"
      >
        <Download aria-hidden="true" />
        <span class="utility-label">有可用更新</span>
        <ChevronRight :size="14" aria-hidden="true" />
      </button>
      <div class="library-utility-pair">
        <button
          :class="{ selected: activeUtility === 'model' }"
          :aria-current="activeUtility === 'model' ? 'page' : undefined"
          @click="emit('utilities', 'model')"
        >
          <Cpu aria-hidden="true" /><span class="utility-label">模型</span>
        </button>
        <button
          :class="{ selected: activeUtility === 'settings' }"
          :aria-current="activeUtility === 'settings' ? 'page' : undefined"
          @click="emit('utilities', 'settings')"
        >
          <Settings aria-hidden="true" /><span class="utility-label">设置</span>
        </button>
      </div>
    </nav>
  </aside>
</template>

<style scoped>
.draft-row-label {
  flex: 1;
}
.draft-sending {
  display: inline-flex;
  flex-shrink: 0;
  margin-left: auto;
  color: var(--muted);
}
.draft-sending svg {
  animation: draft-sending-spin 1s linear infinite;
}
@keyframes draft-sending-spin {
  to {
    transform: rotate(360deg);
  }
}
@media (prefers-reduced-motion: reduce) {
  .draft-sending svg {
    animation: none;
  }
}

.library-utilities button {
  display: grid;
  grid-template-columns: 16px minmax(0, 1fr) auto;
  align-items: center;
  column-gap: 8px;
  min-height: 36px;
  padding: 8px 10px;
  line-height: 20px;
}
.library-utilities button > svg {
  display: block;
  width: 16px;
  height: 16px;
}
.utility-label {
  display: block;
  line-height: 20px;
}

.library-utility-pair {
  border-top: 1px solid var(--line);
  margin-top: 6px;
  padding-top: 8px;
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 6px;
}
.library-utility-pair > button {
  width: 100%;
}
.library-utilities .library-update-entry {
  color: var(--muted);
}
.library-update-entry > svg:last-child {
  width: 14px;
  height: 14px;
}
.today-cost {
  margin-left: auto;
  font-size: 12px;
  font-weight: 500;
  color: var(--ink);
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
}
</style>
