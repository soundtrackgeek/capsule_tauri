import { afterEach, describe, expect, test } from "vitest";

import {
  clearAiApiKey,
  getAiProviderStatus,
  getAiSettings,
  getCapsuleConfig,
  setAiApiKey,
  setCapsuleConfigValue,
  updateAiSettings,
} from "./backend";

async function resetAiSettings() {
  await updateAiSettings({
    cloudProvider: "gemini",
    geminiModel: "gemini-3.5-flash",
    openaiModel: "gpt-5.4-mini",
    openrouterModel: "moonshotai/kimi-k2.5",
    defaultContextLimit: null,
    defaultSince: null,
    defaultUntil: null,
  });
  await clearAiApiKey("gemini");
  await clearAiApiKey("openai");
  await clearAiApiKey("openrouter");
}

describe("mock Cloud AI settings", () => {
  afterEach(async () => {
    await resetAiSettings();
  });

  test("exposes updated defaults and provider model options", async () => {
    const settings = await getAiSettings();
    const statuses = await getAiProviderStatus();
    const gemini = statuses.find((status) => status.provider === "gemini");
    const openrouter = statuses.find((status) => status.provider === "openrouter");

    expect(settings.cloudProvider).toBe("gemini");
    expect(settings.geminiModel).toBe("gemini-3.5-flash");
    expect(settings.openaiModel).toBe("gpt-5.4-mini");
    expect(settings.openrouterModel).toBe("moonshotai/kimi-k2.5");
    expect(gemini?.availableModels).toEqual([
      "gemini-3.5-flash",
      "gemini-3.1-flash-lite-preview",
    ]);
    expect(openrouter?.availableModels).toEqual([
      "z-ai/glm-5.2",
      "moonshotai/kimi-k2.5",
      "qwen/qwen3.7-plus",
      "deepseek/deepseek-v4-flash",
      "xiaomi/mimo-v2.5",
      "minimax/minimax-m3",
    ]);
    expect(openrouter?.availableModels).not.toContain("z-ai/glm-5.1");
    expect(openrouter?.availableModels).not.toContain("qwen/qwen3.5-397b-a17b");
  });

  test("shows legacy model replacements and saves the replacements", async () => {
    await setCapsuleConfigValue("cloud_provider", "openrouter");
    await setCapsuleConfigValue("gemini_model", "gemini-3-flash-preview");
    await setCapsuleConfigValue("openrouter_model", "qwen/qwen3.5-397b-a17b");
    await setCapsuleConfigValue("ai_chat_context_limit", "all");

    const migrated = await getAiSettings();
    expect(migrated.geminiModel).toBe("gemini-3.5-flash");
    expect(migrated.openrouterModel).toBe("qwen/qwen3.7-plus");
    expect(migrated.defaultContextLimit).toBeNull();

    await updateAiSettings({
      ...migrated,
      defaultSince: "2026-01-01",
      defaultUntil: "2026-12-31",
    });

    const savedConfig = await getCapsuleConfig();
    expect(savedConfig.values).toEqual(
      expect.arrayContaining([
        { key: "gemini_model", value: "gemini-3.5-flash" },
        { key: "openrouter_model", value: "qwen/qwen3.7-plus" },
        { key: "ai_chat_context_limit", value: "all" },
        { key: "ai_chat_context_since", value: "2026-01-01" },
        { key: "ai_chat_context_until", value: "2026-12-31" },
      ]),
    );
  });

  test("redacts provider key status after save and clear", async () => {
    const missing = await getAiProviderStatus();
    expect(missing.find((status) => status.provider === "gemini")?.configured).toBe(false);

    const saved = await setAiApiKey({ provider: "gemini", apiKey: "mock-gemini-secret" });
    expect(saved.providerStatus.configured).toBe(true);
    expect(saved.providerStatus.keySource).toBe("OS credential store");
    expect(JSON.stringify(saved)).not.toContain("mock-gemini-secret");

    const cleared = await clearAiApiKey("gemini");
    expect(cleared.providerStatus.configured).toBe(false);
    expect(JSON.stringify(cleared)).not.toContain("mock-gemini-secret");
  });
});
