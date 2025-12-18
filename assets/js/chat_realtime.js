/**
 * GoAmet Chat Client - Best-in-class Realtime Implementation
 * Motto: Never lower the bar, only raise them.
 *
 * Upgrades applied:
 * - P1.1: Container scroll + new messages pill
 * - P1.2: Touch-friendly bottom sheet action menu
 * - P2.1: safeFetchJson wrapper with timeouts
 * - P2.2: Correct reaction toggle (no DELETE on transient failure)
 * - P2.3: WebSocket reconnect with jitter + permanent offline banner
 * - P3.1: Schedule modal (replaces prompt())
 * - P3.2: DOM APIs for reactions (no innerHTML with user content)
 */

class ChatClient {
  constructor(options) {
    this.root = options.root;
    this.localConversationId = options.localConversationId;
    this.resolvedConversationId = null;
    this.currentUserId = options.currentUserId;
    this.canSend = options.canSend;

    this.messagesContainer = document.getElementById("chat-messages-list");
    this.composer = document.getElementById("chat-composer");
    this.input = document.getElementById("chat-input");
    this.sendBtn = document.getElementById("chat-send");
    this.onlineIndicator = document.getElementById("chat-online-indicator");
    this.offlineNotice = document.getElementById("chat-offline-notice");
    this.scheduleBtn = document.getElementById("chat-schedule");

    this.ws = null;
    this.reconnectAttempts = 0;
    this.maxReconnectAttempts = 10;
    this.reconnectDelay = 1000;

    // P1.1: Scroll state tracking
    this.isNearBottom = true;
    this.newMessagesPillVisible = false;

    // Message cache for reaction toggle state
    this.messages = new Map();

    this.init();
    window.chatClient = this;
  }

  // =========================================================================
  // P2.1: Safe Fetch Wrapper with Timeouts
  // =========================================================================

  async safeFetchJson(url, options = {}) {
    const {
      method = "GET",
      headers = {},
      body,
      timeoutMs = 10000,
    } = options;

    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), timeoutMs);

    try {
      const res = await fetch(url, {
        method,
        headers: {
          "Content-Type": "application/json",
          ...headers,
        },
        body: body ? JSON.stringify(body) : undefined,
        signal: controller.signal,
        credentials: "same-origin",
      });

      clearTimeout(timeoutId);

      // Handle auth failures
      if (res.status === 401 || res.status === 403) {
        this.showToast("Sessie verlopen, log opnieuw in");
        setTimeout(() => { window.location.href = "/login"; }, 2000);
        return { ok: false, status: res.status, authError: true };
      }

      // Parse response
      let data = null;
      const contentType = res.headers.get("content-type");
      if (contentType?.includes("application/json")) {
        try {
          data = await res.json();
        } catch {
          data = null;
        }
      } else {
        data = await res.text();
      }

      return { ok: res.ok, status: res.status, data };
    } catch (e) {
      clearTimeout(timeoutId);

      if (e.name === "AbortError") {
        this.showToast("Verbinding timeout - probeer opnieuw");
        return { ok: false, status: 0, timeout: true };
      }

      this.showToast("Netwerkfout - controleer je verbinding");
      return { ok: false, status: 0, networkError: true };
    }
  }

  // =========================================================================
  // Toast Notifications
  // =========================================================================

  showToast(message, type = "error") {
    const toast = document.createElement("div");
    toast.className = `fixed top-20 left-1/2 -translate-x-1/2 px-6 py-3 rounded-2xl text-[13px] font-black shadow-2xl z-[100] animate-in slide-in-from-top-4 duration-300 ${
      type === "error" ? "bg-red-500 text-white" : "bg-goamet-blue text-white"
    }`;
    toast.textContent = message;
    document.body.appendChild(toast);
    setTimeout(() => {
      toast.style.opacity = "0";
      toast.style.transform = "translate(-50%, -20px)";
      toast.style.transition = "all 0.3s ease";
      setTimeout(() => toast.remove(), 300);
    }, 3000);
  }

  // =========================================================================
  // Initialization
  // =========================================================================

  async init() {
    this.root.setAttribute("data-chat-js", "1");
    this.setupEventListeners();
    this.setupScrollObserver();
    const ok = await this.resolveConversation();
    if (ok) {
      await this.loadMessages();
      this.connect();
    }
  }

  setupEventListeners() {
    if (this.canSend && this.input && this.sendBtn) {
      this.sendBtn.addEventListener("click", () => this.sendMessage());
      if (this.scheduleBtn) this.scheduleBtn.addEventListener("click", () => this.showScheduleModal());
      this.input.addEventListener("keydown", (e) => {
        if (e.key === "Enter" && !e.shiftKey) {
          e.preventDefault();
          this.sendMessage();
        }
      });
      this.input.addEventListener("input", () => {
        this.input.style.height = "auto";
        this.input.style.height = this.input.scrollHeight + "px";
      });
    } else if (this.input) {
      this.input.disabled = true;
      this.sendBtn.disabled = true;
    }

    this.messagesContainer?.addEventListener("click", (e) => {
      // P1.2: Action menu trigger
      const actionTrigger = e.target.closest(".action-trigger");
      if (actionTrigger) {
        e.preventDefault();
        const messageId = actionTrigger.dataset.messageId;
        const isOwn = actionTrigger.closest("[id^='msg-']")?.querySelector(".chat-bubble-me") !== null;
        this.showActionMenu(messageId, isOwn);
        return;
      }

      const reactionBtn = e.target.closest(".reaction-pill");
      if (reactionBtn) { this.toggleReaction(reactionBtn.dataset.messageId, reactionBtn.dataset.emoji); return; }
      const addReactionBtn = e.target.closest(".add-reaction-btn");
      if (addReactionBtn) { this.toggleReaction(addReactionBtn.dataset.messageId, "❤️"); return; }
      const pollOptionBtn = e.target.closest(".poll-option-btn");
      if (pollOptionBtn) { this.votePoll(pollOptionBtn.dataset.messageId, pollOptionBtn.dataset.pollId, pollOptionBtn.dataset.optionId); return; }
      const replyBtn = e.target.closest(".reply-btn");
      if (replyBtn) { this.replyTo(replyBtn.dataset.messageId); return; }
    });
  }

  // =========================================================================
  // P1.1: Scroll Management
  // =========================================================================

  setupScrollObserver() {
    if (!this.messagesContainer) return;

    this.messagesContainer.addEventListener("scroll", () => {
      const { scrollTop, scrollHeight, clientHeight } = this.messagesContainer;
      const distanceFromBottom = scrollHeight - scrollTop - clientHeight;
      this.isNearBottom = distanceFromBottom < 120;

      if (this.isNearBottom && this.newMessagesPillVisible) {
        this.hideNewMessagesPill();
      }
    }, { passive: true });
  }

  scrollToBottom(force = false) {
    if (!this.messagesContainer) return;

    if (force || this.isNearBottom) {
      this.messagesContainer.scrollTo({
        top: this.messagesContainer.scrollHeight,
        behavior: force ? "auto" : "smooth"
      });
      this.hideNewMessagesPill();
    } else {
      this.showNewMessagesPill();
    }
  }

  showNewMessagesPill() {
    if (this.newMessagesPillVisible) return;
    this.newMessagesPillVisible = true;

    let pill = document.getElementById("new-messages-pill");
    if (!pill) {
      pill = document.createElement("button");
      pill.id = "new-messages-pill";
      pill.className = "fixed bottom-24 left-1/2 -translate-x-1/2 z-40 px-4 py-2 " +
        "bg-blue-600 text-white text-sm font-medium rounded-full shadow-lg " +
        "flex items-center gap-2 animate-in fade-in slide-in-from-bottom-2 duration-200";
      pill.innerHTML = '<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">' +
        '<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 14l-7 7m0 0l-7-7m7 7V3"/>' +
        '</svg><span>Nieuwe berichten</span>';
      pill.onclick = () => this.scrollToBottom(true);
      document.body.appendChild(pill);
    }
    pill.classList.remove("hidden");
  }

  hideNewMessagesPill() {
    this.newMessagesPillVisible = false;
    const pill = document.getElementById("new-messages-pill");
    if (pill) pill.classList.add("hidden");
  }

  // =========================================================================
  // P1.2: Touch-Friendly Action Menu (Bottom Sheet)
  // =========================================================================

  showActionMenu(messageId, isOwnMessage) {
    this.hideActionMenu();

    const message = this.messages.get(messageId);
    if (!message) return;

    // Inject styles once
    if (!document.getElementById("action-menu-styles")) {
      const style = document.createElement("style");
      style.id = "action-menu-styles";
      style.textContent = `
        .action-menu-btn {
          display: flex;
          align-items: center;
          gap: 12px;
          width: 100%;
          padding: 16px;
          border-radius: 12px;
          color: white;
          font-size: 16px;
          text-align: left;
          transition: background-color 0.15s;
        }
        .action-menu-btn:hover, .action-menu-btn:focus {
          background-color: rgba(255, 255, 255, 0.1);
          outline: none;
        }
        .action-menu-btn:active {
          background-color: rgba(255, 255, 255, 0.15);
        }
        .safe-area-inset-bottom {
          padding-bottom: max(2rem, env(safe-area-inset-bottom));
        }
      `;
      document.head.appendChild(style);
    }

    // Create overlay
    const overlay = document.createElement("div");
    overlay.id = "action-menu-overlay";
    overlay.className = "fixed inset-0 bg-black/50 z-50 animate-in fade-in duration-150";
    overlay.onclick = () => this.hideActionMenu();

    // Create bottom sheet
    const sheet = document.createElement("div");
    sheet.id = "action-menu-sheet";
    sheet.className = "fixed bottom-0 left-0 right-0 z-50 bg-gray-900/95 backdrop-blur-xl " +
      "rounded-t-3xl p-4 pb-8 safe-area-inset-bottom animate-in slide-in-from-bottom duration-200";
    sheet.setAttribute("role", "dialog");
    sheet.setAttribute("aria-label", "Bericht acties");

    sheet.innerHTML = `
      <div class="w-10 h-1 bg-white/30 rounded-full mx-auto mb-4"></div>
      <div class="space-y-1">
        <button class="action-menu-btn" data-action="reply" data-message-id="${messageId}">
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
              d="M3 10h10a8 8 0 018 8v2M3 10l6 6m-6-6l6-6"/>
          </svg>
          <span>Beantwoorden</span>
        </button>
        <button class="action-menu-btn" data-action="react" data-message-id="${messageId}">
          <span class="text-xl">&#10084;&#65039;</span>
          <span>Reageren</span>
        </button>
      </div>
    `;

    // Event delegation for actions
    sheet.addEventListener("click", (e) => {
      const btn = e.target.closest(".action-menu-btn");
      if (!btn) return;

      const action = btn.dataset.action;
      const msgId = btn.dataset.messageId;

      if (action === "reply") {
        this.replyTo(msgId);
      } else if (action === "react") {
        this.toggleReaction(msgId, "❤️");
      }

      this.hideActionMenu();
    });

    // Keyboard handling
    sheet.addEventListener("keydown", (e) => {
      if (e.key === "Escape") this.hideActionMenu();
    });

    document.body.appendChild(overlay);
    document.body.appendChild(sheet);

    // Focus first button for accessibility
    sheet.querySelector(".action-menu-btn")?.focus();
  }

  hideActionMenu() {
    document.getElementById("action-menu-overlay")?.remove();
    document.getElementById("action-menu-sheet")?.remove();
  }

  // =========================================================================
  // Reply
  // =========================================================================

  async replyTo(messageId) {
    const el = document.getElementById(`msg-${messageId}`);
    if (!el) return;
    const content = el.querySelector(".msg-content")?.textContent || "";
    const preview = document.createElement("div");
    preview.id = "reply-preview";
    preview.className = "flex items-center gap-2 p-2 mb-2 bg-white/5 rounded-xl border-l-4 border-goamet-blue animate-in slide-in-from-left duration-200";
    preview.innerHTML = `
      <div class="flex-1 min-w-0"><div class="text-[10px] font-black text-goamet-blue uppercase">Replying to</div><div class="text-[12px] text-white/60 truncate">${this.escapeHtml(content)}</div></div>
      <button onclick="document.getElementById('reply-preview').remove(); document.getElementById('chat-input').dataset.replyTo=''" class="p-1"><svg class="w-4 h-4 text-white/40" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path></svg></button>
    `;
    const existing = document.getElementById("reply-preview");
    if (existing) existing.remove();
    this.composer.prepend(preview);
    this.input.dataset.replyTo = messageId;
    this.input.focus();
  }

  clearReply() {
    const preview = document.getElementById("reply-preview");
    if (preview) preview.remove();
    if (this.input) this.input.dataset.replyTo = "";
  }

  // =========================================================================
  // Connection Status
  // =========================================================================

  setStatus(status) {
    const isOnline = status === "online";
    if (this.onlineIndicator) this.onlineIndicator.classList.toggle("hidden", isOnline);
    if (this.composer) this.composer.classList.toggle("hidden", !isOnline);
    if (this.offlineNotice) this.offlineNotice.classList.toggle("hidden", isOnline);
  }

  // =========================================================================
  // Conversation Resolution
  // =========================================================================

  async resolveConversation() {
    const result = await this.safeFetchJson(
      `/api/chat/resolve-conversation?local_conversation_id=${encodeURIComponent(this.localConversationId)}`,
      { timeoutMs: 15000 }
    );

    if (result.ok && result.data?.chat_conversation_id) {
      this.resolvedConversationId = result.data.chat_conversation_id;
      return true;
    }
    return false;
  }

  // =========================================================================
  // Load Messages
  // =========================================================================

  async loadMessages() {
    const result = await this.safeFetchJson(
      `/api/chat/conversations/${this.resolvedConversationId}/messages?limit=50`,
      { timeoutMs: 15000 }
    );

    if (!result.ok) return;

    const messages = result.data?.messages || [];
    if (this.messagesContainer) {
      this.messagesContainer.innerHTML = "";
      messages.reverse().forEach(m => {
        this.messages.set(m.id, m);
        this.appendMessage(m, false);
      });
      // Force scroll on initial load
      setTimeout(() => this.scrollToBottom(true), 100);
    }
  }

  // =========================================================================
  // P2.3: WebSocket Connection with Jitter
  // =========================================================================

  async connect() {
    const result = await this.safeFetchJson("/api/chat/ws-ticket", {
      method: "POST",
      body: { conversation_id: this.resolvedConversationId },
      timeoutMs: 10000,
    });

    if (!result.ok) {
      this.handleReconnect();
      return;
    }

    const { ws_url } = result.data || {};
    if (!ws_url) return;

    this.ws = new WebSocket(ws_url);

    this.ws.onopen = () => {
      this.setStatus("online");
      this.reconnectAttempts = 0;
      this.reconnectDelay = 1000;
      this.hidePermanentOfflineBanner();
    };

    this.ws.onclose = () => {
      this.setStatus("offline");
      this.handleReconnect();
    };

    this.ws.onerror = () => {
      this.ws.close();
    };

    this.ws.onmessage = (ev) => this.handleWsMessage(ev);
  }

  handleReconnect() {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      this.showPermanentOfflineBanner();
      return;
    }

    this.reconnectAttempts++;

    // Add jitter: ±20% randomization to prevent thundering herd
    const jitter = 0.8 + Math.random() * 0.4;
    const delay = Math.floor(this.reconnectDelay * jitter);

    this.updateConnectionStatus("reconnecting", this.reconnectAttempts);

    setTimeout(() => this.connect(), delay);
    this.reconnectDelay = Math.min(this.reconnectDelay * 2, 30000);
  }

  showPermanentOfflineBanner() {
    document.getElementById("offline-banner")?.remove();

    const banner = document.createElement("div");
    banner.id = "offline-banner";
    banner.className = "fixed top-0 left-0 right-0 z-50 bg-red-600 text-white " +
      "px-4 py-3 flex items-center justify-between";
    banner.style.paddingTop = "max(0.75rem, env(safe-area-inset-top))";
    banner.innerHTML = `
      <span class="text-sm font-medium">Verbinding verbroken</span>
      <button id="manual-reconnect" class="px-3 py-1.5 bg-white/20 hover:bg-white/30
        rounded-lg text-sm font-medium transition">
        Opnieuw verbinden
      </button>
    `;

    document.body.prepend(banner);

    document.getElementById("manual-reconnect")?.addEventListener("click", () => {
      this.reconnectAttempts = 0;
      this.reconnectDelay = 1000;
      banner.remove();
      this.connect();
    });
  }

  hidePermanentOfflineBanner() {
    document.getElementById("offline-banner")?.remove();
  }

  updateConnectionStatus(status, attempt = 0) {
    const indicator = document.getElementById("connection-status");
    if (!indicator) return;

    if (status === "connected") {
      indicator.classList.add("hidden");
    } else if (status === "reconnecting") {
      indicator.classList.remove("hidden");
      indicator.textContent = `Verbinden... (${attempt}/${this.maxReconnectAttempts})`;
    }
  }

  // =========================================================================
  // WebSocket Message Handling
  // =========================================================================

  handleWsMessage(ev) {
    try {
      const data = JSON.parse(ev.data);
      if (data.type === "new_message") {
        this.messages.set(data.message.id, data.message);
        this.appendMessage(data.message, true);
      } else if (data.type === "message_updated") {
        this.messages.set(data.message.id, data.message);
        this.updateMessageInUI(data.message);
      } else if (data.type === "reaction_added" || data.type === "reaction_removed") {
        this.updateReactionsInUI(data.message_id, data.reactions);
      }
    } catch (e) {}
  }

  // =========================================================================
  // Send Message
  // =========================================================================

  async sendMessage() {
    const content = (this.input.value || "").trim();
    if (!content || !this.canSend) return;

    const replyTo = this.input.dataset.replyTo;

    // Optimistic UI: clear input immediately
    this.input.value = "";
    this.input.style.height = "auto";
    this.clearReply();
    this.sendBtn.disabled = true;

    const endpoint = replyTo
      ? `/api/chat/conversations/${this.resolvedConversationId}/messages/${replyTo}/reply`
      : `/api/chat/conversations/${this.resolvedConversationId}/messages`;

    const result = await this.safeFetchJson(endpoint, {
      method: "POST",
      body: { content },
      timeoutMs: 15000,
    });

    if (!result.ok) {
      // Rollback: restore input
      this.input.value = content;
      if (!result.authError) {
        this.showToast("Bericht kon niet worden verzonden");
      }
    } else if (result.data) {
      this.messages.set(result.data.id, result.data);
      this.appendMessage(result.data, true);
    }

    this.sendBtn.disabled = false;
  }

  // =========================================================================
  // P2.2: Correct Reaction Toggle
  // =========================================================================

  async toggleReaction(messageId, emoji) {
    if (!this.canSend) return;

    const message = this.messages.get(messageId);
    if (!message) return;

    // Determine current state from message cache
    const reactions = message.reactions || {};
    const currentCount = reactions[emoji] || 0;
    // For simplicity, we assume if count > 0, user might have reacted
    // Real implementation would track per-user reactions
    const isAdding = currentCount === 0 || !message._myReactions?.has(emoji);

    // Optimistic UI update
    this.updateReactionOptimistic(messageId, emoji, isAdding);

    const url = `/api/chat/conversations/${this.resolvedConversationId}/messages/${messageId}/reactions`;

    let result;
    if (isAdding) {
      result = await this.safeFetchJson(url, {
        method: "POST",
        body: { emoji },
        timeoutMs: 8000,
      });

      // Only attempt DELETE on 409 Conflict (already exists)
      if (!result.ok && result.status === 409) {
        result = await this.safeFetchJson(
          `${url}/${encodeURIComponent(emoji)}`,
          { method: "DELETE", timeoutMs: 8000 }
        );
      }
    } else {
      result = await this.safeFetchJson(
        `${url}/${encodeURIComponent(emoji)}`,
        { method: "DELETE", timeoutMs: 8000 }
      );
    }

    // Rollback on failure (except auth errors which redirect)
    if (!result.ok && !result.authError) {
      this.updateReactionOptimistic(messageId, emoji, !isAdding);
      this.showToast("Reactie kon niet worden bijgewerkt");
    }
  }

  updateReactionOptimistic(messageId, emoji, isAdding) {
    const el = document.getElementById(`msg-${messageId}`);
    if (!el) return;

    const container = el.querySelector(".reactions-container");
    if (!container) return;

    const pill = container.querySelector(`.reaction-pill[data-emoji="${emoji}"]`);

    if (pill) {
      const countEl = pill.querySelector(".count");
      let count = parseInt(countEl?.textContent || "0", 10);
      count = isAdding ? count + 1 : Math.max(0, count - 1);

      if (count === 0) {
        pill.remove();
      } else if (countEl) {
        countEl.textContent = count;
      }
    } else if (isAdding) {
      // Add new pill using DOM API (P3.2: no innerHTML)
      const newPill = document.createElement("button");
      newPill.className = "reaction-pill bg-white/10 hover:bg-white/20 px-2 py-0.5 rounded-full text-[11px] flex items-center gap-1 transition";
      newPill.dataset.messageId = messageId;
      newPill.dataset.emoji = emoji;

      const emojiSpan = document.createElement("span");
      emojiSpan.textContent = emoji;

      const countSpan = document.createElement("span");
      countSpan.className = "count font-bold";
      countSpan.textContent = "1";

      newPill.appendChild(emojiSpan);
      newPill.appendChild(countSpan);
      container.appendChild(newPill);
    }
  }

  // =========================================================================
  // Poll Voting
  // =========================================================================

  async votePoll(messageId, pollId, optionId) {
    if (!this.canSend) return;

    const result = await this.safeFetchJson(
      `/api/chat/conversations/${this.resolvedConversationId}/polls/${pollId}/vote`,
      {
        method: "POST",
        body: { option_id: optionId },
        timeoutMs: 10000,
      }
    );

    if (!result.ok && !result.authError) {
      this.showToast("Stem kon niet worden uitgebracht");
    }
  }

  // =========================================================================
  // P3.1: Schedule Modal (replaces prompt())
  // =========================================================================

  showScheduleModal() {
    const content = (this.input.value || "").trim();
    if (!content) {
      this.showToast("Typ eerst een bericht");
      return;
    }

    this.hideScheduleModal();

    // Default: now + 1 hour, rounded to nearest 5 minutes
    const defaultDate = new Date(Date.now() + 3600000);
    defaultDate.setMinutes(Math.round(defaultDate.getMinutes() / 5) * 5, 0, 0);
    const defaultValue = defaultDate.toISOString().slice(0, 16);

    const overlay = document.createElement("div");
    overlay.id = "schedule-overlay";
    overlay.className = "fixed inset-0 bg-black/60 z-50 flex items-center justify-center p-4 " +
      "animate-in fade-in duration-150";

    overlay.innerHTML = `
      <div id="schedule-modal" class="w-full max-w-sm bg-gray-900/95 backdrop-blur-xl
        rounded-2xl p-6 shadow-2xl animate-in zoom-in-95 duration-200"
        role="dialog" aria-labelledby="schedule-title">

        <h2 id="schedule-title" class="text-lg font-semibold text-white mb-4">
          Bericht inplannen
        </h2>

        <div class="mb-4">
          <label for="schedule-datetime" class="block text-sm text-white/70 mb-2">
            Wanneer versturen?
          </label>
          <input
            type="datetime-local"
            id="schedule-datetime"
            value="${defaultValue}"
            min="${new Date().toISOString().slice(0, 16)}"
            class="w-full px-4 py-3 bg-white/10 border border-white/20 rounded-xl
              text-white focus:outline-none focus:ring-2 focus:ring-blue-500
              focus:border-transparent"
          />
          <p id="schedule-error" class="mt-2 text-sm text-red-400 hidden"></p>
        </div>

        <div class="flex gap-3">
          <button id="schedule-cancel" class="flex-1 px-4 py-3 bg-white/10
            hover:bg-white/20 text-white rounded-xl font-medium transition">
            Annuleren
          </button>
          <button id="schedule-confirm" class="flex-1 px-4 py-3 bg-blue-600
            hover:bg-blue-700 text-white rounded-xl font-medium transition">
            Inplannen
          </button>
        </div>
      </div>
    `;

    document.body.appendChild(overlay);

    const input = document.getElementById("schedule-datetime");
    input?.focus();

    document.getElementById("schedule-cancel")?.addEventListener("click", () => {
      this.hideScheduleModal();
    });

    overlay.addEventListener("click", (e) => {
      if (e.target === overlay) this.hideScheduleModal();
    });

    document.getElementById("schedule-confirm")?.addEventListener("click", async () => {
      const datetimeInput = document.getElementById("schedule-datetime");
      const errorEl = document.getElementById("schedule-error");
      const selectedDate = new Date(datetimeInput.value);

      // Validation
      if (isNaN(selectedDate.getTime())) {
        errorEl.textContent = "Selecteer een geldige datum en tijd";
        errorEl.classList.remove("hidden");
        return;
      }

      if (selectedDate <= new Date()) {
        errorEl.textContent = "Kies een tijdstip in de toekomst";
        errorEl.classList.remove("hidden");
        return;
      }

      // Max 30 days ahead
      const maxDate = new Date(Date.now() + 30 * 24 * 60 * 60 * 1000);
      if (selectedDate > maxDate) {
        errorEl.textContent = "Maximaal 30 dagen vooruit inplannen";
        errorEl.classList.remove("hidden");
        return;
      }

      await this.scheduleMessage(content, selectedDate.toISOString());
      this.hideScheduleModal();
    });

    overlay.addEventListener("keydown", (e) => {
      if (e.key === "Escape") this.hideScheduleModal();
    });
  }

  hideScheduleModal() {
    document.getElementById("schedule-overlay")?.remove();
  }

  async scheduleMessage(content, scheduledFor) {
    const result = await this.safeFetchJson(
      `/api/chat/conversations/${this.resolvedConversationId}/scheduled`,
      {
        method: "POST",
        body: { content, scheduled_for: scheduledFor },
        timeoutMs: 15000,
      }
    );

    if (result.ok) {
      this.input.value = "";
      this.input.style.height = "auto";
      this.showToast("Bericht ingepland!", "success");
    }
  }

  // =========================================================================
  // Message Rendering
  // =========================================================================

  appendMessage(m, scroll = true) {
    if (document.getElementById(`msg-${m.id}`)) {
      this.updateMessageInUI(m);
      return;
    }
    const isMe = m.sender_id === this.currentUserId;
    const el = this.renderMessage(m, isMe);
    this.messagesContainer.appendChild(el);
    if (scroll) this.scrollToBottom(false);
  }

  updateMessageInUI(m) {
    const el = document.getElementById(`msg-${m.id}`);
    if (el) {
      const contentEl = el.querySelector(".msg-content");
      if (contentEl) contentEl.textContent = m.content;
      this.updateReactionsInUI(m.id, m.reactions);
      if (m.poll) this.updatePollInUI(m.id, m.poll);
    }
  }

  // P3.2: Use DOM APIs instead of innerHTML for user content
  updateReactionsInUI(messageId, reactions = {}, parentEl = null) {
    const el = parentEl || document.getElementById(`msg-${messageId}`);
    if (!el) return;
    const container = el.querySelector(".reactions-container");
    if (!container) return;

    // Clear existing
    container.innerHTML = "";

    if (reactions) {
      Object.entries(reactions).forEach(([emoji, count]) => {
        if (count <= 0) return;

        const pill = document.createElement("button");
        pill.className = "reaction-pill bg-white/10 hover:bg-white/20 px-2 py-0.5 rounded-full text-[11px] flex items-center gap-1 transition";
        pill.dataset.messageId = messageId;
        pill.dataset.emoji = emoji;

        // Safe: createElement + textContent, NO innerHTML for user content
        const emojiSpan = document.createElement("span");
        emojiSpan.textContent = emoji;

        const countSpan = document.createElement("span");
        countSpan.className = "count font-bold";
        countSpan.textContent = count;

        pill.appendChild(emojiSpan);
        pill.appendChild(countSpan);
        container.appendChild(pill);
      });
    }
  }

  updatePollInUI(messageId, poll) {
    const el = document.getElementById(`msg-${messageId}`);
    if (!el) return;
    const container = el.querySelector(".poll-container");
    if (container) container.innerHTML = this.renderPollHtml(messageId, poll);
  }

  renderPollHtml(messageId, poll) {
    const totalVotes = poll.options.reduce((sum, opt) => sum + opt.vote_count, 0);
    return `
      <div class="mt-3 p-4 rounded-2xl bg-white/5 border border-white/10 space-y-3">
        <h4 class="text-sm font-bold text-white/90">${this.escapeHtml(poll.question)}</h4>
        <div class="space-y-2">
          ${poll.options.map(opt => {
            const percentage = totalVotes > 0 ? (opt.vote_count / totalVotes) * 100 : 0;
            return `
              <button class="poll-option-btn w-full text-left relative group overflow-hidden rounded-xl border border-white/5 hover:border-goamet-blue/30 transition active:scale-[0.99]"
                      data-message-id="${messageId}" data-poll-id="${poll.id}" data-option-id="${opt.id}">
                <div class="absolute inset-0 bg-goamet-blue/10 transition-all duration-500" style="width: ${percentage}%"></div>
                <div class="relative px-3 py-2.5 flex justify-between items-center text-[13px]">
                  <span class="font-semibold text-white/80">${this.escapeHtml(opt.text)}</span>
                  <span class="text-[11px] font-black text-white/40 tabular-nums">${opt.vote_count}</span>
                </div>
              </button>`;
          }).join("")}
        </div>
        <div class="flex justify-between items-center text-[10px] font-bold text-white/30 uppercase tracking-tighter">
          <span>${totalVotes} votes</span>${poll.is_anonymous ? "<span>Anonymous</span>" : ""}
        </div>
      </div>`;
  }

  renderMessage(m, isMe) {
    const row = document.createElement("div");
    row.id = `msg-${m.id}`;
    row.className = `flex flex-col w-full mb-4 ${isMe ? "items-end" : "items-start"}`;
    const bubbleClass = isMe ? "chat-bubble-me" : "chat-bubble-them";
    const time = new Date(m.created_at).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
    let replyHtml = m.reply_to_message_id ? `<div class="mb-2 p-2 bg-white/5 rounded-lg border-l-2 border-white/20 text-[12px] opacity-60 truncate">Replied to message</div>` : "";

    // P1.2: Action trigger button (touch-friendly, replaces hover-only controls)
    const actionTrigger = `
      <button class="action-trigger p-2 -m-2 rounded-full hover:bg-white/10
        active:bg-white/20 transition touch-manipulation"
        data-message-id="${m.id}"
        aria-label="Bericht acties"
        aria-haspopup="dialog">
        <svg class="w-5 h-5 text-white/50" fill="currentColor" viewBox="0 0 24 24">
          <circle cx="12" cy="6" r="2"/><circle cx="12" cy="12" r="2"/><circle cx="12" cy="18" r="2"/>
        </svg>
      </button>
    `;

    row.innerHTML = `
      <div class="flex items-end gap-2">
        ${!isMe ? actionTrigger : ""}
        <div class="max-w-[85%] px-4 py-3 shadow-sm ${bubbleClass} relative group">
          ${replyHtml}<div class="msg-content text-[15px] leading-relaxed whitespace-pre-wrap">${this.escapeHtml(m.content)}</div>
          <div class="poll-container"></div>
          <div class="mt-1 flex items-center justify-end gap-1.5 opacity-40 text-[10px] font-bold">
            ${m.is_pinned ? '<svg class="w-3 h-3 text-goamet-blue" fill="currentColor" viewBox="0 0 20 20"><path d="M11.013 1.427a1.75 1.75 0 012.474 0l1.086 1.086a1.75 1.75 0 010 2.474l-8.61 8.61c-.21.21-.47.367-.754.45l-3.273.962a.75.75 0 01-.948-.948l.962-3.273c.084-.283.24-.544.45-.754l8.61-8.61z"></path></svg>' : ""}
            <span>${time}</span>${isMe ? '<svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M5 13l4 4L19 7"></path></svg>' : ""}
          </div>
        </div>
        ${isMe ? actionTrigger : ""}
      </div>
      <div class="reactions-container flex flex-wrap gap-1 mt-1 px-1"></div>`;

    this.updateReactionsInUI(m.id, m.reactions, row);
    if (m.poll) row.querySelector(".poll-container").innerHTML = this.renderPollHtml(m.id, m.poll);
    return row;
  }

  escapeHtml(s) {
    return String(s || "")
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;")
      .replace(/'/g, "&#039;");
  }
}

// =========================================================================
// Auto-initialization
// =========================================================================

document.addEventListener("DOMContentLoaded", () => {
  const root = document.querySelector("[data-chat-conversation-id]");
  if (root) {
    new ChatClient({
      root: root,
      localConversationId: root.getAttribute("data-chat-conversation-id"),
      currentUserId: root.getAttribute("data-chat-current-user-id"),
      canSend: root.getAttribute("data-chat-can-send") === "1"
    });
  }
});
