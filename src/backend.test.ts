import { afterEach, describe, expect, test } from "vitest";

import {
  cancelAiChatStream,
  clearAiApiKey,
  deleteAiConversation,
  getAiConversation,
  getAiProviderStatus,
  getAiSettings,
  getCapsuleConfig,
  listAiConversations,
  previewAiChatContext,
  retryAiChatStream,
  setAiApiKey,
  setCapsuleConfigValue,
  startAiChatStream,
  subscribeAiChatEvents,
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
      model: "gemini-3.5-flash",
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
    expect(detail.model).toBe("gemini-3.5-flash");
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
      model: "qwen/qwen3.7-plus",
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
      model: "qwen/qwen3.7-plus",
      contextEntryUuids: cancelled.scopeIdentifiers,
    });
    await waitUntil(() => completed);
    unsubscribe();
    const retried = await getAiConversation(start.conversationId);
    expect(retried.messages.at(-1)?.status).toBe("complete");
    expect(retried.model).toBe("qwen/qwen3.7-plus");
  });
});
