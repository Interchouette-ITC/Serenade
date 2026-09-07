/*!
 * Clitorine - light UX helpers beside Bootstrap + Quill (MyFeed demo only).
 * Keep the name in sources; not a product marketing surface.
 */
(function () {
  "use strict";

  const MAX_BODY = 2000;
  const MAX_IMAGE_BYTES = 200 * 1024;
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
    const hidden = byId("image_data");
    const file = input.files && input.files[0];
    if (!file || !hidden) {
      return;
    }
    if (file.size > MAX_IMAGE_BYTES) {
      window.alert("Image must be under 200KB for this demo.");
      input.value = "";
      return;
    }
    if (!file.type.startsWith("image/")) {
      window.alert("Please choose an image file.");
      input.value = "";
      return;
    }
    const reader = new FileReader();
    reader.onload = function () {
      const data = String(reader.result || "");
      hidden.value = data;
      if (preview) {
        preview.innerHTML =
          '<img class="img-fluid rounded border" alt="preview" src="' +
          data.replace(/"/g, "&quot;") +
          '" />';
      }
    };
    reader.readAsDataURL(file);
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
      notice.className = "alert alert-success mt-2 mb-0 py-2 small";
      notice.setAttribute("data-clitorine-notice", "1");
      notice.setAttribute("role", "status");
      anchor.appendChild(notice);
    }
    notice.textContent = message;
  }

  function bindAjaxForms() {
    document
      .querySelectorAll("form[data-clitorine-ajax]")
      .forEach(function (form) {
        form.addEventListener("submit", function (event) {
          event.preventDefault();
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
                const badge =
                  article && article.querySelector("[data-like-count]");
                const count = Number.parseInt(text.trim(), 10);
                if (badge && Number.isFinite(count) && count > 0) {
                  badge.textContent = String(count) + " likes";
                }
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
            .catch(function () {
              form.submit();
            });
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
    document
      .querySelectorAll("[data-clitorine-delete]")
      .forEach(function (btn) {
        btn.addEventListener("click", function () {
          const action = btn.getAttribute("data-delete-action") || "";
          const token = btn.getAttribute("data-delete-token") || "";
          confirmForm.setAttribute("action", action);
          csrfSlot.innerHTML =
            '<input type="hidden" name="_token" value="' +
            token.replace(/"/g, "&quot;") +
            '" />';
          bootstrap.Modal.getOrCreateInstance(modalEl).show();
        });
      });
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

    const imageInput = byId("media_upload");
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
