import { afterEach, describe, expect, test } from "vitest";

import {
  cancelAiChatStream,
  clearAiApiKey,
  createMood,
  deleteAiConversation,
  deleteMood,
  getAiConversation,
  getAiProviderStatus,
  getAiSettings,
  getCapsuleConfig,
  getPathSettings,
  listAiConversations,
  listMoods,
  previewAiChatContext,
  retryAiChatStream,
  setAiApiKey,
  setCapsuleConfigValue,
  setPathSettings,
  startAiChatStream,
  subscribeAiChatEvents,
  updateAiSettings,
  updateMoodSentiment,
} from "./backend";

describe("mock mood catalog", () => {
  test("creates a reusable mood and edits its sentiment", async () => {
    const name = "reflective-test";
    const existing = await listMoods();
    if (existing.moods.some((mood) => mood.name === name)) {
      await deleteMood({ name });
    }

    try {
      const created = await createMood({ name, sentimentScore: 0.35 });
      expect(created.moods).toEqual(
        expect.arrayContaining([
          expect.objectContaining({ name, entryCount: 0, sentimentScore: 0.35 }),
        ]),
      );

      const updated = await updateMoodSentiment({ name, sentimentScore: 0.7 });
      expect(updated.moods.find((mood) => mood.name === name)?.sentimentScore).toBe(0.7);
    } finally {
      const moods = await listMoods();
      if (moods.moods.some((mood) => mood.name === name)) {
        await deleteMood({ name });
      }
    }
  });
});

describe("mock Windows startup setting", () => {
  afterEach(async () => {
    await setPathSettings({ startWithWindows: false });
  });

  test("reports the active start-with-Windows registration", async () => {
    expect((await getPathSettings()).startWithWindows).toBe(false);

    const enabled = await setPathSettings({ startWithWindows: true });
    expect(enabled.startWithWindows).toBe(true);
    expect((await getPathSettings()).startWithWindows).toBe(true);
  });
});

describe("mock word target settings", () => {
  afterEach(async () => {
    await setPathSettings({
      wordTargetEnabled: false,
      wordTarget: 500,
      gauntletModeEnabled: false,
    });
  });

  test("persists word targets and clamps invalid target sizes", async () => {
    const enabled = await setPathSettings({
      wordTargetEnabled: true,
      wordTarget: 0,
      gauntletModeEnabled: true,
    });

    expect(enabled).toMatchObject({
      wordTargetEnabled: true,
      wordTarget: 1,
      gauntletModeEnabled: true,
    });
    expect(await getPathSettings()).toMatchObject(enabled);
  });
});

async function resetAiSettings() {
  await updateAiSettings({
    cloudProvider: "gemini",
    geminiModel: "gemini-4-hydrogen",
    openaiModel: "gpt-6-luna",
    openrouterModel: "deepseek/deepseek-v4.1-flash-1017",
    defaultContextLimit: null,
    defaultSince: null,
    defaultUntil: null,
  });
  await clearAiApiKey("gemini");
  await clearAiApiKey("openai");
  await clearAiApiKey("openrouter");
}

async function resetAiChats() {
  const response = await listAiConversations();
  await Promise.all(
    response.conversations.map((conversation) => deleteAiConversation(conversation.id)),
  );
}

async function waitUntil(predicate: () => boolean, timeoutMs = 2_000) {
  const startedAt = Date.now();
  while (!predicate()) {
    if (Date.now() - startedAt > timeoutMs) {
      throw new Error("Timed out waiting for mock stream event.");
    }
    await new Promise((resolve) => window.setTimeout(resolve, 25));
  }
}

describe("mock Cloud AI settings", () => {
  afterEach(async () => {
    await resetAiSettings();
  });

  test("exposes updated defaults and provider model options", async () => {
    const settings = await getAiSettings();
    const statuses = await getAiProviderStatus();
    const gemini = statuses.find((status) => status.provider === "gemini");
    const openai = statuses.find((status) => status.provider === "openai");
    const openrouter = statuses.find((status) => status.provider === "openrouter");

    expect(settings.cloudProvider).toBe("gemini");
    expect(settings.geminiModel).toBe("gemini-4-hydrogen");
    expect(settings.openaiModel).toBe("gpt-6-luna");
    expect(settings.openrouterModel).toBe("deepseek/deepseek-v4.1-flash-1017");
    expect(gemini?.availableModels).toEqual([
      "gemini-4-hydrogen",
      "gemini-3.8-flash",
      "gemini-3.5-flash-lite",
    ]);
    expect(gemini?.availableModels).not.toContain("gemini-3.1-flash-lite-preview");
    expect(openai?.availableModels).toEqual(["gpt-6-luna", "gpt-6.1-terra"]);
    expect(openai?.availableModels).not.toContain("gpt-5.4-nano");
    expect(openrouter?.availableModels).toEqual([
      "deepseek/deepseek-v4.1-flash-1017",
      "z-ai/glm-5.3",
      "moonshotai/kimi-k3.1-pro",
      "~x-ai/grok-latest",
      "qwen/qwen4-max",
      "xiaomi/mimo-v2.6-pro",
      "minimax/minimax-m3.1-flash",
    ]);
    expect(openrouter?.availableModels).not.toContain("moonshotai/kimi-k2.5");
    expect(openrouter?.availableModels).not.toContain("x-ai/grok-4.5");
  });

  test("shows legacy model replacements and saves the replacements", async () => {
    await setCapsuleConfigValue("cloud_provider", "openrouter");
    await setCapsuleConfigValue("gemini_model", "gemini-3-flash-preview");
    await setCapsuleConfigValue("openai_model", "gpt-5.4-mini");
    await setCapsuleConfigValue("openrouter_model", "qwen/qwen3.5-397b-a17b");
    await setCapsuleConfigValue("ai_chat_context_limit", "all");

    const migrated = await getAiSettings();
    expect(migrated.geminiModel).toBe("gemini-4-hydrogen");
    expect(migrated.openaiModel).toBe("gpt-6-luna");
    expect(migrated.openrouterModel).toBe("qwen/qwen4-max");
    expect(migrated.defaultContextLimit).toBeNull();

    await updateAiSettings({
      ...migrated,
      defaultSince: "2026-01-01",
      defaultUntil: "2026-12-31",
    });

    const savedConfig = await getCapsuleConfig();
    expect(savedConfig.values).toEqual(
      expect.arrayContaining([
        { key: "gemini_model", value: "gemini-4-hydrogen" },
        { key: "openrouter_model", value: "qwen/qwen4-max" },
        { key: "ai_chat_context_limit", value: "all" },
        { key: "ai_chat_context_since", value: "2026-01-01" },
        { key: "ai_chat_context_until", value: "2026-12-31" },
      ]),
    );
  });

  test("migrates retired November 2026 models to their successors", async () => {
    await setCapsuleConfigValue("gemini_model", "gemini-3.1-flash-lite-preview");
    await setCapsuleConfigValue("openai_model", "gpt-5.4-nano");
    await setCapsuleConfigValue("openrouter_model", "moonshotai/kimi-k2.5");

    const migrated = await getAiSettings();
    expect(migrated.geminiModel).toBe("gemini-3.5-flash-lite");
    expect(migrated.openaiModel).toBe("gpt-6-luna");
    expect(migrated.openrouterModel).toBe("moonshotai/kimi-k3.1-pro");
  });

  test("migrates the direct Grok 4.5 slug to the OpenRouter Grok Latest alias", async () => {
    await setCapsuleConfigValue("cloud_provider", "openrouter");
    await setCapsuleConfigValue("openrouter_model", "x-ai/grok-4.5");

    const migrated = await getAiSettings();
    expect(migrated.openrouterModel).toBe("~x-ai/grok-latest");

    await updateAiSettings(migrated);
    const savedConfig = await getCapsuleConfig();
    expect(savedConfig.values).toEqual(
      expect.arrayContaining([{ key: "openrouter_model", value: "~x-ai/grok-latest" }]),
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

describe("mock AI chat", () => {
  afterEach(async () => {
    await resetAiChats();
    await resetAiSettings();
  });

  test("previews context and streams a complete response", async () => {
    const preview = await previewAiChatContext({
      message: "Capsule Tauri",
      scope: "search",
      scopeIdentifiers: [],
      contextFilters: null,
      contextLimit: 2,
      since: null,
      until: null,
      contextEntryUuids: null,
    });
    expect(preview.entries.length).toBeGreaterThan(0);

    let completed = false;
    let streamedContent = "";
    const unsubscribe = await subscribeAiChatEvents({
      chunk: (event) => {
        streamedContent = event.content;
      },
      complete: () => {
        completed = true;
      },
    });

    const start = await startAiChatStream({
      message: "What stands out about Capsule Tauri?",
      cloudProvider: "gemini",
      model: "gemini-4-hydrogen",
      scope: "search",
      scopeIdentifiers: [],
      contextFilters: null,
      contextLimit: 2,
      since: null,
      until: null,
      contextEntryUuids: preview.entries.map((entry) => entry.uuid),
    });

    await waitUntil(() => completed);
    unsubscribe();
    const detail = await getAiConversation(start.conversationId);
    expect(streamedContent).toContain("mock streamer");
    expect(detail.model).toBe("gemini-4-hydrogen");
    expect(detail.messages.at(-1)?.status).toBe("complete");
    expect(JSON.stringify(detail)).not.toContain("mock-gemini-secret");
  });

  test("cancels and retries a mock stream", async () => {
    let interrupted = false;
    let completed = false;
    const unsubscribe = await subscribeAiChatEvents({
      interrupted: () => {
        interrupted = true;
      },
      complete: () => {
        completed = true;
      },
    });

    const start = await startAiChatStream({
      message: "Summarize the Codex workflow note",
      cloudProvider: "openrouter",
      model: "qwen/qwen4-max",
      scope: "search",
      scopeIdentifiers: [],
      contextFilters: null,
      contextLimit: 1,
      since: null,
      until: null,
      contextEntryUuids: null,
    });
    await cancelAiChatStream(start.streamId);
    await waitUntil(() => interrupted);
    const cancelled = await getAiConversation(start.conversationId);
    expect(cancelled.messages.at(-1)?.status).toBe("interrupted");

    await retryAiChatStream({
      conversationId: start.conversationId,
      cloudProvider: "openrouter",
      model: "qwen/qwen4-max",
      contextEntryUuids: cancelled.scopeIdentifiers,
    });
    await waitUntil(() => completed);
    unsubscribe();
    const retried = await getAiConversation(start.conversationId);
    expect(retried.messages.at(-1)?.status).toBe("complete");
    expect(retried.model).toBe("qwen/qwen4-max");
  });
});
