import { create } from "zustand"
import { persist, createJSONStorage } from "zustand/middleware"
import { nanoid } from "nanoid"
import type { Rule } from "@/types"

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
  addRule: (rule: Omit<Rule, "id">) => void
  updateRule: (id: string, updates: Partial<Rule>) => void
  removeRule: (id: string) => void
  toggleRule: (id: string) => void
  resetRules: () => void
}

export const useDetectionRulesStore = create<DetectionRulesStore>()(
  persist(
    (set) => ({
      rules: defaultRules,
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
    }),
    {
      name: "linch-redact-detection-rules",
      storage: createJSONStorage(() => localStorage),
      version: 1,
      migrate: (persistedState) => {
        if (!persistedState || typeof persistedState !== "object") {
          return persistedState
        }
        const state = persistedState as { rules?: Rule[] }
        if (!Array.isArray(state.rules)) {
          return persistedState
        }
        const defaultsById = new Map(defaultRules.map((rule) => [rule.id, rule]))
        const fixedRules = state.rules.map((rule) => {
          const def = defaultsById.get(rule.id)
          if (!def) return rule
          return {
            ...rule,
            name: def.name,
            pattern: def.pattern,
            ruleType: def.ruleType,
          }
        })
        return { ...state, rules: fixedRules }
      },
    }
  )
)

export const builtinRuleIds = new Set(defaultRules.map((rule) => rule.id))
