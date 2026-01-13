import { create } from "zustand"
import { subscribeWithSelector } from "zustand/middleware"
import { invoke } from "@tauri-apps/api/core"
import { nanoid } from "nanoid"
import type { Rule, DetectionHit, DetectionHitsByFile } from "@/types"

// 按文件存储已添加为遮罩的命中索引
interface AddedHitsByFile {
  [fileId: string]: Set<number>
}

const defaultRules: Rule[] = [
  {
    id: "id_card_cn",
    name: "身份证号",
    ruleType: "regex",
    pattern: "\\d{17}[\\dXx]",
    enabled: true,
  },
  {
    id: "phone_cn",
    name: "手机号",
    ruleType: "regex",
    pattern: "1[3-9]\\d{9}",
    enabled: true,
  },
  {
    id: "email",
    name: "邮箱",
    ruleType: "regex",
    pattern: "[\\w.+-]+@[\\w.-]+\\.\\w{2,}",
    enabled: false,
  },
  {
    id: "bank_card",
    name: "银行卡号",
    ruleType: "regex",
    pattern: "\\d{16,19}",
    enabled: false,
  },
]

interface DetectionRulesStore {
  rules: Rule[]
  initialized: boolean

  // 按文件存储的检测结果
  hitsByFile: DetectionHitsByFile
  addedHitsByFile: AddedHitsByFile

  // 规则管理
  loadRules: () => Promise<void>
  addRule: (rule: Omit<Rule, "id">) => void
  updateRule: (id: string, updates: Partial<Rule>) => void
  removeRule: (id: string) => void
  toggleRule: (id: string) => void
  resetRules: () => void

  // 检测结果管理
  getHits: (fileId: string) => DetectionHit[]
  getAddedHits: (fileId: string) => Set<number>
  setHits: (fileId: string, hits: DetectionHit[]) => void
  clearHits: (fileId: string) => void
  markHitAdded: (fileId: string, hitIndex: number) => void
  unmarkHitAdded: (fileId: string, hitIndex: number) => void
  markAllHitsAdded: (fileId: string) => void
}

export const useDetectionRulesStore = create<DetectionRulesStore>()(
  subscribeWithSelector((set, get) => ({
    rules: defaultRules,
    initialized: false,
    hitsByFile: {},
    addedHitsByFile: {},

    loadRules: async () => {
      try {
        const savedRules = await invoke<Rule[]>("load_detection_rules")
        if (savedRules && savedRules.length > 0) {
          // 合并：保留用户的启用状态，但使用最新的内置规则定义
          const defaultsById = new Map(defaultRules.map((rule) => [rule.id, rule]))
          const savedById = new Map(savedRules.map((rule) => [rule.id, rule]))

          // 合并内置规则（使用保存的启用状态）
          const mergedRules: Rule[] = defaultRules.map((def) => {
            const saved = savedById.get(def.id)
            if (saved) {
              return { ...def, enabled: saved.enabled }
            }
            return def
          })

          // 添加用户自定义规则
          for (const rule of savedRules) {
            if (!defaultsById.has(rule.id)) {
              mergedRules.push(rule)
            }
          }

          set({ rules: mergedRules, initialized: true })
        } else {
          set({ initialized: true })
        }
      } catch (err) {
        console.error("加载检测规则失败:", err)
        set({ initialized: true })
      }
    },

    addRule: (rule) => {
      const newRule: Rule = { ...rule, id: `custom_${nanoid(8)}` }
      set((state) => ({ rules: [...state.rules, newRule] }))
    },

    updateRule: (id, updates) => {
      set((state) => ({
        rules: state.rules.map((rule) => (rule.id === id ? { ...rule, ...updates } : rule)),
      }))
    },

    removeRule: (id) => {
      set((state) => ({ rules: state.rules.filter((rule) => rule.id !== id) }))
    },

    toggleRule: (id) => {
      set((state) => ({
        rules: state.rules.map((rule) =>
          rule.id === id ? { ...rule, enabled: !rule.enabled } : rule
        ),
      }))
    },

    resetRules: () => set({ rules: defaultRules }),

    // 检测结果管理
    getHits: (fileId) => {
      return get().hitsByFile[fileId] ?? []
    },

    getAddedHits: (fileId) => {
      return get().addedHitsByFile[fileId] ?? new Set()
    },

    setHits: (fileId, hits) => {
      set((state) => ({
        hitsByFile: {
          ...state.hitsByFile,
          [fileId]: hits,
        },
        addedHitsByFile: {
          ...state.addedHitsByFile,
          [fileId]: new Set(),
        },
      }))
    },

    clearHits: (fileId) => {
      set((state) => {
        const newHits = { ...state.hitsByFile }
        const newAdded = { ...state.addedHitsByFile }
        delete newHits[fileId]
        delete newAdded[fileId]
        return {
          hitsByFile: newHits,
          addedHitsByFile: newAdded,
        }
      })
    },

    markHitAdded: (fileId, hitIndex) => {
      set((state) => {
        const currentAdded = state.addedHitsByFile[fileId] ?? new Set()
        const newAdded = new Set(currentAdded)
        newAdded.add(hitIndex)
        return {
          addedHitsByFile: {
            ...state.addedHitsByFile,
            [fileId]: newAdded,
          },
        }
      })
    },

    unmarkHitAdded: (fileId, hitIndex) => {
      set((state) => {
        const currentAdded = state.addedHitsByFile[fileId] ?? new Set()
        const newAdded = new Set(currentAdded)
        newAdded.delete(hitIndex)
        return {
          addedHitsByFile: {
            ...state.addedHitsByFile,
            [fileId]: newAdded,
          },
        }
      })
    },

    markAllHitsAdded: (fileId) => {
      set((state) => {
        const hits = state.hitsByFile[fileId] ?? []
        const newAdded = new Set(hits.map((_, idx) => idx))
        return {
          addedHitsByFile: {
            ...state.addedHitsByFile,
            [fileId]: newAdded,
          },
        }
      })
    },
  }))
)

// 订阅规则变化，自动保存到文件
useDetectionRulesStore.subscribe(
  (state) => state.rules,
  (rules, prevRules) => {
    const { initialized } = useDetectionRulesStore.getState()
    // 只有在初始化完成后才保存（避免覆盖已保存的数据）
    if (initialized && rules !== prevRules) {
      invoke("save_detection_rules", { rules }).catch((err) => {
        console.error("保存检测规则失败:", err)
      })
    }
  }
)

export const builtinRuleIds = new Set(defaultRules.map((rule) => rule.id))
