import assert from "node:assert/strict";
import { test } from "node:test";
import { hasComposerDraft } from "../assets/clitorine/composer.js";

import {
  protectedCard,
  retainedInteraction,
} from "../assets/clitorine/feed-cards.js";

test("link-only composer drafts block navigation to new posts", () => {
  const link = { value: "" };
  globalThis.document = { getElementById: () => link };
  try {
    assert.equal(hasComposerDraft(), false);
    link.value = "https://www.phoronix.com/news/article";
    assert.equal(hasComposerDraft(), true);
    link.value = "   ";
    assert.equal(hasComposerDraft(), false);
  } finally {
    delete globalThis.document;
  }
});

test("off-screen focused players no longer pin a card", () => {
  const player = { matches: (selector) => selector === "iframe" };
  globalThis.document = { activeElement: player };
  globalThis.window = { innerHeight: 800 };
  const card = {
    dataset: {},
    contains: (element) => element === player,
    querySelector: () => null,
    querySelectorAll: () => [],
    getBoundingClientRect: () => ({ top: 100, bottom: 200 }),
  };

  assert.equal(protectedCard(card), true);
  card.getBoundingClientRect = () => ({ top: -200, bottom: -1 });
  assert.equal(protectedCard(card), false);
  card.querySelectorAll = (selector) =>
    selector === "textarea" ? [{ value: "draft" }] : [];
  assert.equal(retainedInteraction(card), true);
  assert.equal(protectedCard(card), true);
});
