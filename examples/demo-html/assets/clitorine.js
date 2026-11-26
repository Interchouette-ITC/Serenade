/*!
 * Clitorine - light UX helpers beside Bootstrap + Quill (MyFeed demo only).
 * Keep the name in sources; not a product marketing surface.
 */
(function () {
  "use strict";

  const MAX_BODY = 2000;
  const MAX_IMAGE_BYTES = 1024 * 1024;
  const EMOJI_ICON =
    '<svg viewBox="0 0 18 18" aria-hidden="true">' +
    '<circle class="ql-stroke" cx="9" cy="9" r="7"></circle>' +
    '<circle class="ql-fill" cx="6.5" cy="7.5" r="1.1"></circle>' +
    '<circle class="ql-fill" cx="11.5" cy="7.5" r="1.1"></circle>' +
    '<path class="ql-stroke" d="M6.2,11.2c.9,1.1,1.9,1.6,2.8,1.6s1.9-.5,2.8-1.6"></path>' +
    "</svg>";

  let quill = null;

  function byId(id) {
    return document.getElementById(id);
  }

  function feedLabels() {
    return (
      byId("feed-controller") || document.querySelector("[data-feed-labels]")
    );
  }

  function openComposer() {
    const panel = byId("composer-panel");
    if (!panel || typeof bootstrap === "undefined") {
      return;
    }
    bootstrap.Collapse.getOrCreateInstance(panel, { toggle: false }).show();
    window.setTimeout(function () {
      ensureQuill();
      if (quill) {
        quill.focus();
      }
    }, 180);
  }

  function closeComposer() {
    const panel = byId("composer-panel");
    if (!panel || typeof bootstrap === "undefined") {
      return;
    }
    bootstrap.Collapse.getOrCreateInstance(panel, { toggle: false }).hide();
    const trigger = byId("composer-trigger");
    if (trigger) {
      trigger.value = "";
    }
    hideEmojiPanel();
  }

  function syncEditor() {
    const hidden = byId("body");
    const counter = byId("composer-count");
    if (!quill || !hidden) {
      return;
    }
    const text = quill.getText().replace(/\n$/, "");
    if (text.length > MAX_BODY) {
      quill.deleteText(MAX_BODY, text.length);
    }
    hidden.value = quill.root.innerHTML;
    if (counter) {
      const len = quill.getText().replace(/\n$/, "").length;
      counter.textContent = String(len) + "/" + String(MAX_BODY);
    }
  }

  function insertEmoji(glyph) {
    ensureQuill();
    if (!quill || !glyph) {
      return;
    }
    const range = quill.getSelection(true);
    const index = range ? range.index : Math.max(0, quill.getLength() - 1);
    quill.insertText(index, glyph, "user");
    quill.setSelection(index + glyph.length, 0, "user");
    syncEditor();
    hideEmojiPanel();
  }

  function emojiPanel() {
    return document.querySelector("[data-clitorine-emoji-panel]");
  }

  function hideEmojiPanel() {
    const panel = emojiPanel();
    if (panel) {
      panel.hidden = true;
    }
  }

  function toggleEmojiPanel() {
    const panel = emojiPanel();
    if (!panel) {
      return;
    }
    panel.hidden = !panel.hidden;
  }

  function onImagePick(input) {
    const preview = byId("media-preview");
    const file = input.files && input.files[0];
    if (!file) {
      return;
    }
    if (file.size > MAX_IMAGE_BYTES) {
      window.alert("Image must be 1 MB or smaller for this demo.");
      input.value = "";
      return;
    }
    if (!file.type.startsWith("image/")) {
      window.alert("Please choose an image file.");
      input.value = "";
      return;
    }
    if (preview) {
      const url = URL.createObjectURL(file);
      preview.innerHTML =
        '<img class="img-fluid rounded border" alt="preview" src="' +
        url +
        '" />';
    }
  }

  function ensureQuill() {
    const host = byId("composer-quill");
    if (!host || quill || typeof Quill === "undefined") {
      return;
    }

    const icons = Quill.import("ui/icons");
    icons.emoji = EMOJI_ICON;

    quill = new Quill("#composer-quill", {
      theme: "snow",
      placeholder: "Share a story, a thought, a clip…",
      modules: {
        toolbar: {
          container: [
            ["bold", "italic", "underline"],
            [{ list: "ordered" }, { list: "bullet" }],
            ["blockquote", "link"],
            ["emoji"],
          ],
          handlers: {
            emoji: function () {
              toggleEmojiPanel();
            },
          },
        },
      },
    });

    const initial = host.getAttribute("data-initial-html");
    if (initial) {
      quill.clipboard.dangerouslyPasteHTML(initial);
    }

    quill.on("text-change", syncEditor);
    syncEditor();
  }

  function bindEmojiButtons() {
    document.querySelectorAll("[data-clitorine-emoji]").forEach(function (btn) {
      btn.addEventListener("click", function (event) {
        event.preventDefault();
        insertEmoji(btn.getAttribute("data-clitorine-emoji") || "");
      });
    });
  }

  function showInlineNotice(anchor, message) {
    if (!anchor) {
      return;
    }
    let notice = anchor.querySelector("[data-clitorine-notice]");
    if (!notice) {
      notice = document.createElement("div");
      notice.setAttribute("data-clitorine-notice", "1");
      notice.setAttribute("role", "status");
      anchor.appendChild(notice);
    }
    notice.className = "alert alert-success mt-2 mb-0 py-2 small";
    notice.textContent = message;
  }

  function bindAjaxForms() {
    document.addEventListener("submit", function (event) {
      const form = event.target.closest("form[data-clitorine-ajax]");
      if (!form) {
        return;
      }
      event.preventDefault();
      if (form.dataset.busy === "1") {
        return;
      }
      form.dataset.busy = "1";
      const kind = form.getAttribute("data-clitorine-ajax") || "";
      const params = new URLSearchParams(new FormData(form));
      fetch(form.action, {
        method: "POST",
        body: params.toString(),
        credentials: "same-origin",
        headers: {
          "X-MyFeed-Ajax": "1",
          "Content-Type": "application/x-www-form-urlencoded",
        },
      })
        .then(function (response) {
          if (!response.ok) {
            throw new Error("request failed");
          }
          return response.text();
        })
        .then(function (text) {
          if (kind === "like") {
            const article = form.closest("article");
            const badge = article && article.querySelector("[data-like-count]");
            const count = Number.parseInt(text.trim(), 10);
            if (!Number.isFinite(count) || count <= 0 || !badge) {
              throw new Error("Invalid like response");
            }
            badge.textContent = String(count) + " likes";
            return;
          }
          if (kind === "comment") {
            const wrap = form.parentElement;
            form.reset();
            showInlineNotice(
              wrap,
              text.trim() || "Comment sent. It will appear after approval.",
            );
          }
        })
        .catch(function (error) {
          console.error("MyFeed action failed", error);
          const labels = feedLabels();
          showInlineNotice(form.parentElement, labels.dataset.error);
          const notice = form.parentElement.querySelector(
            "[data-clitorine-notice]",
          );
          if (notice) {
            notice.className = "alert alert-danger mt-2 mb-0 py-2 small";
          }
        })
        .finally(function () {
          delete form.dataset.busy;
        });
    });
  }

  function bindDeleteModal() {
    const modalEl = byId("delete-post-modal");
    const confirmForm = byId("delete-post-confirm-form");
    const csrfSlot = byId("delete-post-csrf");
    if (
      !modalEl ||
      !confirmForm ||
      !csrfSlot ||
      typeof bootstrap === "undefined"
    ) {
      return;
    }
    document.addEventListener("click", function (event) {
      const btn = event.target.closest("[data-clitorine-delete]");
      if (!btn) {
        return;
      }
      const action = btn.getAttribute("data-delete-action") || "";
      const token = btn.getAttribute("data-delete-token") || "";
      confirmForm.setAttribute("action", action);
      csrfSlot.innerHTML =
        '<input type="hidden" name="_token" value="' +
        token.replace(/"/g, "&quot;") +
        '" />';
      bootstrap.Modal.getOrCreateInstance(modalEl).show();
    });
  }

  function bindComments() {
    document.addEventListener("click", async function (event) {
      const link = event.target.closest("[data-comments-load]");
      if (!link) {
        return;
      }
      event.preventDefault();
      const article = link.closest("article");
      if (article.dataset.commentsBusy === "1") {
        return;
      }
      article.dataset.commentsBusy = "1";
      const status = article.querySelector("[data-comment-status]");
      const labels = feedLabels();
      status.textContent = labels.dataset.loading;
      try {
        const response = await fetch(link.href, {
          headers: { "X-MyFeed-Ajax": "1" },
          credentials: "same-origin",
        });
        if (!response.ok) {
          throw new Error("HTTP " + response.status);
        }
        const template = document.createElement("template");
        template.innerHTML = await response.text();
        const next = template.content.querySelector("[data-comment-window]");
        if (!next || next.querySelectorAll("[data-comment-id]").length > 10) {
          throw new Error("Invalid comment window");
        }
        const old = article.querySelector("[data-comment-window]");
        const offset = link.getBoundingClientRect().top;
        old.replaceWith(next);
        const control = next.querySelector("[data-comments-load]");
        if (control) {
          control.focus({ preventScroll: true });
          window.scrollBy({
            top: control.getBoundingClientRect().top - offset,
            behavior: "instant",
          });
        }
        status.textContent = "";
      } catch (error) {
        console.error("MyFeed comment loading failed", error);
        status.textContent = labels.dataset.error;
      } finally {
        delete article.dataset.commentsBusy;
      }
    });
  }

  function bindFeed() {
    const controller = byId("feed-controller");
    const frame = byId("feed-window");
    if (!controller || !frame) {
      return;
    }
    const wall = byId("wall");
    const topSpacer = frame.querySelector('[data-feed-spacer="top"]');
    const bottomSpacer = frame.querySelector('[data-feed-spacer="bottom"]');
    const olderLink = frame.querySelector('[data-feed-load="older"]');
    const newerLink = frame.querySelector('[data-feed-load="newer"]');
    const end = frame.querySelector("[data-feed-end]");
    const status = byId("feed-status");
    const newPosts = byId("feed-new-posts");
    const retry = document.createElement("button");
    retry.type = "button";
    retry.className = "btn btn-primary feed-retry";
    retry.textContent = olderLink.textContent;
    retry.hidden = true;
    status.after(retry);
    const pageSize = 20;
    const maxCards = 60;
    const hour = 60 * 60 * 1000;
    let groups = [Array.from(wall.querySelectorAll("article"))];
    let ceiling = frame.dataset.ceiling;
    let hasOlder = frame.dataset.hasOlder === "true";
    let hasNewer = frame.dataset.hasNewer === "true";
    let topHeight = 0;
    let bottomHeight = 0;
    let request = null;
    let failedDirection = null;
    let suspended = false;
    let restoring = false;
    let generation = 0;
    let lastScroll = window.scrollY;
    let direction = "older";
    let raf = null;
    let pollTimer = null;
    let pollRequest = null;
    let lastPoll = Date.now();
    let anchor = null;
    let leaving = false;
    const stateKey = "myfeedPosition";

    function cards() {
      return Array.from(wall.querySelectorAll("article"));
    }

    function visibleAnchor() {
      const mounted = cards();
      const card = mounted.find(function (node) {
        const rect = node.getBoundingClientRect();
        return rect.bottom > 0 && rect.top < window.innerHeight;
      });
      return card
        ? { id: card.id, offset: card.getBoundingClientRect().top }
        : null;
    }

    function correctAnchor(saved) {
      if (!saved) {
        return;
      }
      const card = byId(saved.id);
      if (card) {
        const change = card.getBoundingClientRect().top - saved.offset;
        if (Math.abs(change) > 0.5) {
          window.scrollBy({ top: change, behavior: "instant" });
          lastScroll = window.scrollY;
        }
      }
      anchor = visibleAnchor();
    }

    function protectedCard(card) {
      return card.contains(document.activeElement) || retainedInteraction(card);
    }

    function retainedInteraction(card) {
      return (
        card.dataset.commentsBusy === "1" ||
        card.querySelector('[data-busy="1"], [data-comments-busy="1"]') ||
        Array.from(card.querySelectorAll("textarea")).some(function (input) {
          return input.value.trim() !== "";
        }) ||
        Array.from(card.querySelectorAll("video")).some(function (video) {
          return !video.paused;
        }) ||
        card.dataset.mediaPinned === "1"
      );
    }

    function protectedGroup(group) {
      return group.some(function (card) {
        const rect = card.getBoundingClientRect();
        return (
          protectedCard(card) ||
          (rect.bottom > 0 && rect.top < window.innerHeight)
        );
      });
    }

    // Cross-origin players do not expose their playback state; focus pins them.
    wall.addEventListener("focusin", function (event) {
      if (event.target.matches("iframe")) {
        event.target.closest("article").dataset.mediaPinned = "1";
      }
    });
    window.addEventListener("blur", function () {
      const focused = document.activeElement;
      if (focused && focused.matches("#wall iframe")) {
        focused.closest("article").dataset.mediaPinned = "1";
      }
    });
    wall.addEventListener("click", function (event) {
      const release = event.target.closest("[data-release-media]");
      if (release) {
        delete release.closest("article").dataset.mediaPinned;
        release.blur();
        suspended = false;
        considerLoading();
      }
    });

    function groupHeight(group) {
      if (!group.length) {
        return 0;
      }
      const first = group[0].getBoundingClientRect();
      const last = group[group.length - 1].getBoundingClientRect();
      return (
        last.bottom -
        first.top +
        Number.parseFloat(getComputedStyle(wall).rowGap)
      );
    }

    function setSpacers() {
      topSpacer.style.height = Math.max(0, topHeight) + "px";
      bottomSpacer.style.height = Math.max(0, bottomHeight) + "px";
    }

    function updateControls() {
      const mounted = cards();
      const first = mounted[0];
      const last = mounted[mounted.length - 1];
      newerLink.hidden = !hasNewer;
      olderLink.hidden = !hasOlder;
      end.hidden = hasOlder;
      newerLink.href =
        "/?after=" + (first ? first.id.slice(5) : "0") + "&ceiling=" + ceiling;
      olderLink.href =
        "/?before=" + (last ? last.id.slice(5) : "0") + "&ceiling=" + ceiling;
      frame.dataset.ceiling = ceiling;
      frame.dataset.hasOlder = String(hasOlder);
      frame.dataset.hasNewer = String(hasNewer);
      frame.dataset.newest = first ? first.id.slice(5) : "0";
      frame.dataset.oldest = last ? last.id.slice(5) : "0";
      if (observer) {
        observer.disconnect();
        if (first) {
          observer.observe(first);
        }
        if (last && last !== first) {
          observer.observe(last);
        }
      }
    }

    function savePosition() {
      const saved = visibleAnchor();
      if (!saved || restoring || leaving) {
        return;
      }
      const previous = history.state || {};
      history.replaceState(
        Object.assign({}, previous, {
          [stateKey]: {
            id: saved.id.slice(5),
            offset: saved.offset,
            ceiling: ceiling,
          },
        }),
        "",
      );
    }

    function evict(group) {
      group.forEach(function (card) {
        if (resizeObserver) {
          resizeObserver.unobserve(card);
        }
        card.querySelectorAll("video").forEach(function (video) {
          video.pause();
          video.removeAttribute("src");
          video.load();
        });
        card.querySelectorAll("iframe").forEach(function (iframe) {
          iframe.src = "about:blank";
        });
        card.remove();
      });
    }

    async function fetchWindow(url, signal) {
      const response = await fetch(url, {
        headers: { "X-MyFeed-Ajax": "1" },
        credentials: "same-origin",
        signal: signal,
      });
      if (!response.ok) {
        throw new Error("HTTP " + response.status);
      }
      const template = document.createElement("template");
      template.innerHTML = await response.text();
      const next = template.content.querySelector("#feed-window");
      if (
        !next ||
        next.querySelectorAll("article").length > pageSize ||
        !/^\d+$/.test(next.dataset.ceiling)
      ) {
        throw new Error("Invalid feed window");
      }
      return {
        frame: next,
        missing: response.headers.get("x-myfeed-missing") === "1",
      };
    }

    async function load(which, manual) {
      if (
        request ||
        restoring ||
        (!manual && suspended) ||
        (which === "older" ? !hasOlder : !hasNewer)
      ) {
        return;
      }
      const mounted = cards();
      const trim =
        mounted.length + pageSize > maxCards
          ? which === "older"
            ? groups[0]
            : groups[groups.length - 1]
          : null;
      if (trim && protectedGroup(trim)) {
        status.textContent = controller.dataset.blocked;
        failedDirection = which;
        retry.hidden = false;
        suspended = true;
        return;
      }
      suspended = false;
      retry.hidden = true;
      request = new AbortController();
      const activeRequest = request;
      const activeGeneration = generation;
      status.textContent = controller.dataset.loading;
      try {
        const result = await fetchWindow(
          which === "older" ? olderLink.href : newerLink.href,
          activeRequest.signal,
        );
        if (activeGeneration !== generation) {
          return;
        }
        const saved = visibleAnchor();
        const existing = new Set(
          mounted.map(function (card) {
            return card.id;
          }),
        );
        const added = Array.from(
          result.frame.querySelectorAll("article"),
        ).filter(function (card) {
          return !existing.has(card.id);
        });
        if (trim && protectedGroup(trim)) {
          status.textContent = controller.dataset.blocked;
          suspended = true;
          failedDirection = which;
          retry.hidden = false;
          return;
        }
        if (!added.length) {
          if (which === "older") {
            hasOlder = false;
          } else {
            hasNewer = false;
          }
        } else {
          if (trim) {
            const height = groupHeight(trim);
            if (which === "older") {
              groups.shift();
              topHeight += height;
              hasNewer = true;
            } else {
              groups.pop();
              bottomHeight += height;
              hasOlder = true;
            }
            evict(trim);
          }
          if (which === "older") {
            wall.append(...added);
            groups.push(added);
            bottomHeight = Math.max(0, bottomHeight - groupHeight(added));
            hasOlder = result.frame.dataset.hasOlder === "true";
            if (!hasOlder) {
              bottomHeight = 0;
            }
          } else {
            wall.prepend(...added);
            groups.unshift(added);
            topHeight = Math.max(0, topHeight - groupHeight(added));
            hasNewer = result.frame.dataset.hasNewer === "true";
            if (!hasNewer) {
              topHeight = 0;
            }
          }
          added.forEach(function (card) {
            if (resizeObserver) {
              resizeObserver.observe(card);
            }
          });
        }
        status.textContent = "";
        setSpacers();
        updateControls();
        correctAnchor(saved);
        failedDirection = null;
      } catch (error) {
        if (error.name !== "AbortError") {
          console.error("MyFeed loading failed", error);
          status.textContent = controller.dataset.error;
          failedDirection = which;
          retry.hidden = false;
          suspended = true;
        }
      } finally {
        if (request === activeRequest) {
          request = null;
        }
        savePosition();
      }
    }

    function considerLoading() {
      if (!observer || request || restoring || suspended || document.hidden) {
        return;
      }
      const mounted = cards();
      if (!mounted.length) {
        return;
      }
      if (
        direction === "older" &&
        hasOlder &&
        mounted[mounted.length - 1].getBoundingClientRect().top <
          window.innerHeight + 600
      ) {
        load("older", false);
      } else if (
        direction === "newer" &&
        hasNewer &&
        mounted[0].getBoundingClientRect().bottom > -600
      ) {
        load("newer", false);
      }
    }

    const observer =
      typeof IntersectionObserver === "undefined"
        ? null
        : new IntersectionObserver(considerLoading, {
            rootMargin: "600px 0px",
          });
    const resizeObserver =
      typeof ResizeObserver === "undefined"
        ? null
        : new ResizeObserver(function () {
            if (!restoring && window.scrollY === lastScroll) {
              correctAnchor(anchor);
            } else if (!restoring) {
              anchor = visibleAnchor();
            }
          });

    frame.addEventListener("click", function (event) {
      const link = event.target.closest("[data-feed-load]");
      if (link) {
        event.preventDefault();
        direction = link.dataset.feedLoad;
        load(direction, true);
      }
    });
    retry.addEventListener("click", function () {
      if (failedDirection) {
        load(failedDirection, true);
      }
    });
    window.addEventListener(
      "scroll",
      function () {
        if (raf !== null) {
          return;
        }
        raf = requestAnimationFrame(function () {
          raf = null;
          if (window.scrollY !== lastScroll) {
            direction = window.scrollY >= lastScroll ? "older" : "newer";
            lastScroll = window.scrollY;
          }
          anchor = visibleAnchor();
          considerLoading();
        });
      },
      { passive: true },
    );

    async function restore(saved) {
      if (request) {
        request.abort();
        request = null;
      }
      generation += 1;
      const activeGeneration = generation;
      restoring = true;
      suspended = false;
      const activeRequest = new AbortController();
      request = activeRequest;
      status.textContent = controller.dataset.loading;
      try {
        const result = await fetchWindow(
          "/?around=" +
            saved.id +
            (saved.ceiling ? "&ceiling=" + saved.ceiling : ""),
          activeRequest.signal,
        );
        if (generation !== activeGeneration) {
          return;
        }
        const oldCards = cards();
        if (oldCards.some(retainedInteraction)) {
          status.textContent = controller.dataset.blocked;
          suspended = true;
          return;
        }
        oldCards.forEach(function (card) {
          evict([card]);
        });
        const nextCards = Array.from(result.frame.querySelectorAll("article"));
        wall.replaceChildren(...nextCards);
        groups = [nextCards];
        topHeight = 0;
        bottomHeight = 0;
        ceiling = result.frame.dataset.ceiling;
        hasOlder = result.frame.dataset.hasOlder === "true";
        hasNewer = result.frame.dataset.hasNewer === "true";
        status.textContent = result.missing ? controller.dataset.missing : "";
        setSpacers();
        updateControls();
        const target = byId("post-" + saved.id) || nextCards[0];
        if (target) {
          target.scrollIntoView({ block: "start", behavior: "instant" });
          window.scrollBy({
            top: -(Number.isFinite(saved.offset) ? saved.offset : 0),
            behavior: "instant",
          });
        }
        anchor = visibleAnchor();
        nextCards.forEach(function (card) {
          if (resizeObserver) {
            resizeObserver.observe(card);
          }
        });
      } catch (error) {
        if (error.name !== "AbortError") {
          console.error("MyFeed position restoration failed", error);
          status.textContent = controller.dataset.error;
          suspended = true;
          // A normal navigation retains a usable no-JavaScript recovery path.
          newerLink.href = "/?around=" + saved.id;
          newerLink.hidden = false;
          newerLink.dataset.restore = saved.id;
        }
      } finally {
        if (generation === activeGeneration) {
          restoring = false;
        }
        if (request === activeRequest) {
          request = null;
        }
        lastScroll = window.scrollY;
      }
    }

    frame.addEventListener(
      "click",
      function (event) {
        const link = event.target.closest("[data-restore]");
        if (link) {
          event.preventDefault();
          event.stopImmediatePropagation();
          restore({ id: link.dataset.restore, offset: 0 });
          delete link.dataset.restore;
        }
      },
      true,
    );

    function clearPoll() {
      clearTimeout(pollTimer);
      pollTimer = null;
    }

    function schedulePoll() {
      clearPoll();
      if (!document.hidden) {
        pollTimer = setTimeout(
          checkNewPosts,
          Math.max(0, hour - (Date.now() - lastPoll)),
        );
      }
    }

    async function checkNewPosts() {
      if (document.hidden || pollRequest) {
        return;
      }
      lastPoll = Date.now();
      pollRequest = new AbortController();
      try {
        const response = await fetch("/feed/latest", {
          signal: pollRequest.signal,
          credentials: "same-origin",
        });
        if (!response.ok) {
          throw new Error("HTTP " + response.status);
        }
        const latest = (await response.text()).trim();
        if (!/^\d+$/.test(latest)) {
          throw new Error("Invalid latest post id");
        }
        newPosts.hidden = BigInt(latest) <= BigInt(ceiling);
      } catch (error) {
        if (error.name !== "AbortError") {
          console.error("MyFeed new-post check failed", error);
          status.textContent = controller.dataset.error;
        }
      } finally {
        pollRequest = null;
        schedulePoll();
      }
    }

    document.addEventListener("visibilitychange", function () {
      if (document.hidden) {
        clearPoll();
        savePosition();
        if (pollRequest) {
          pollRequest.abort();
        }
      } else {
        schedulePoll();
      }
    });
    document.addEventListener(
      "click",
      function (event) {
        const link = event.target.closest("a[href]");
        if (link && !link.matches("[data-feed-load], [data-comments-load]")) {
          savePosition();
          if (
            link.origin === location.origin &&
            link.pathname !== location.pathname
          ) {
            leaving = true;
          }
        }
      },
      true,
    );
    document.addEventListener(
      "submit",
      function (event) {
        savePosition();
        if (!event.target.matches("[data-clitorine-ajax]")) {
          leaving = true;
        }
      },
      true,
    );
    newPosts.addEventListener("click", function (event) {
      if (
        cards().some(protectedCard) ||
        (quill && quill.getText().trim() !== "")
      ) {
        event.preventDefault();
        status.textContent = controller.dataset.blocked;
      } else {
        history.replaceState(null, "");
      }
    });
    window.addEventListener("pagehide", function () {
      savePosition();
      clearPoll();
      if (request) {
        request.abort();
      }
      if (pollRequest) {
        pollRequest.abort();
      }
    });
    window.addEventListener("pageshow", function (event) {
      schedulePoll();
      if (event.persisted) {
        leaving = false;
        request = null;
        restoring = false;
        anchor = visibleAnchor();
        updateControls();
      }
    });
    window.addEventListener("popstate", function () {
      const saved = history.state && history.state[stateKey];
      if (saved) {
        restore(saved);
      }
    });
    window.addEventListener("hashchange", function () {
      const match = /^#post-([1-9]\d*)$/.exec(location.hash);
      if (match) {
        restore({ id: match[1], offset: 0 });
      }
    });

    updateControls();
    cards().forEach(function (card) {
      if (resizeObserver) {
        resizeObserver.observe(card);
      }
    });
    const hash = /^#post-([1-9]\d*)$/.exec(location.hash);
    const navigation = performance.getEntriesByType("navigation")[0];
    const saved = history.state && history.state[stateKey];
    if (
      saved &&
      navigation &&
      (navigation.type === "back_forward" || navigation.type === "reload")
    ) {
      restore(saved);
    } else if (hash) {
      restore({ id: hash[1], offset: 0 });
    } else {
      anchor = visibleAnchor();
      considerLoading();
    }
    schedulePoll();
  }

  function bind() {
    const trigger = byId("composer-trigger");
    if (trigger) {
      trigger.addEventListener("focus", openComposer);
      trigger.addEventListener("click", openComposer);
    }

    const cancel = byId("composer-cancel");
    if (cancel) {
      cancel.addEventListener("click", function (event) {
        event.preventDefault();
        closeComposer();
      });
    }

    bindEmojiButtons();
    bindAjaxForms();
    bindDeleteModal();
    bindComments();
    bindFeed();

    const imageInput = byId("image");
    if (imageInput) {
      imageInput.addEventListener("change", function () {
        onImagePick(imageInput);
      });
    }

    const form = document.querySelector("form.composer-form");
    if (form) {
      form.addEventListener("submit", function () {
        ensureQuill();
        syncEditor();
      });
    }

    if (byId("composer-quill") && !byId("composer-panel")) {
      ensureQuill();
    } else if (document.body.dataset.composerOpen === "1") {
      openComposer();
    }
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", bind);
  } else {
    bind();
  }
})();
