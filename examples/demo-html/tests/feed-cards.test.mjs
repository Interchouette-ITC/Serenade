import assert from "node:assert/strict";
import { test } from "node:test";

import {
  protectedCard,
  retainedInteraction,
} from "../assets/clitorine/feed-cards.js";

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
