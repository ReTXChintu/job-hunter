/**
 * In-memory connection registry: who is online, and routing of frames
 * between the one desktop connection and any mobile connections for the
 * same account. Direct port of `crates/job-hunter-relay/src/hub.rs` --
 * this is pure plumbing, it never looks inside a frame's payload, only at
 * which device sent it and which device(s) it should go to.
 */
import type { DeviceKind } from "./models.js";

/** The minimal surface the hub needs from a live socket -- kept abstract so
 * tests can register plain in-memory senders instead of real WebSockets. */
export interface FrameSender {
  send(text: string): void;
  close(): void;
}

interface Connection {
  deviceId: string;
  sender: FrameSender;
}

interface UserChannels {
  desktop: Connection | null;
  mobiles: Connection[];
}

export interface RegisterOutcome {
  desktopOnline: boolean;
}

export class Hub {
  private readonly users = new Map<string, UserChannels>();

  private channelsFor(userId: string): UserChannels {
    let c = this.users.get(userId);
    if (!c) {
      c = { desktop: null, mobiles: [] };
      this.users.set(userId, c);
    }
    return c;
  }

  /**
   * Registers a live connection. A second desktop connection for the same
   * account replaces (and closes) the previous one -- only one desktop is
   * meant to be online per account at a time. Returns whether a desktop is
   * online right after registering, so the caller can send a mobile client
   * an immediate presence snapshot.
   */
  register(userId: string, deviceId: string, kind: DeviceKind, sender: FrameSender): RegisterOutcome {
    const channels = this.channelsFor(userId);
    if (kind === "desktop") {
      const old = channels.desktop;
      channels.desktop = { deviceId, sender };
      if (old) old.sender.close();
      broadcastPresence(channels.mobiles, true);
    } else {
      channels.mobiles = channels.mobiles.filter((c) => c.deviceId !== deviceId);
      channels.mobiles.push({ deviceId, sender });
    }
    return { desktopOnline: channels.desktop !== null };
  }

  /** Removes a connection. If it was the desktop's, every connected mobile
   * is told it just went offline. No-ops if already replaced (fast reconnect). */
  unregister(userId: string, deviceId: string, kind: DeviceKind): void {
    const channels = this.users.get(userId);
    if (!channels) return;
    if (kind === "desktop") {
      if (channels.desktop?.deviceId === deviceId) {
        channels.desktop = null;
        broadcastPresence(channels.mobiles, false);
      }
    } else {
      channels.mobiles = channels.mobiles.filter((c) => c.deviceId !== deviceId);
    }
    if (!channels.desktop && channels.mobiles.length === 0) {
      this.users.delete(userId);
    }
  }

  isDesktopOnline(userId: string): boolean {
    return this.users.get(userId)?.desktop !== null && this.users.get(userId)?.desktop !== undefined;
  }

  /** Whether the specific device currently has a live connection. */
  isOnline(userId: string, deviceId: string): boolean {
    const channels = this.users.get(userId);
    if (!channels) return false;
    return channels.desktop?.deviceId === deviceId || channels.mobiles.some((m) => m.deviceId === deviceId);
  }

  /** Forwards a frame from a mobile device to the connected desktop. Returns
   * false if no desktop is connected. */
  routeToDesktop(userId: string, text: string): boolean {
    const desktop = this.users.get(userId)?.desktop;
    if (!desktop) return false;
    desktop.sender.send(text);
    return true;
  }

  /** Forwards a frame from the desktop to every connected mobile device for
   * the same account (pushes, and desktop replies to mobile-initiated
   * requests -- each mobile ignores ids it did not send). */
  routeToMobiles(userId: string, text: string): void {
    const channels = this.users.get(userId);
    if (!channels) return;
    for (const conn of channels.mobiles) conn.sender.send(text);
  }

  /** Forces a specific device's live socket closed (e.g. on revocation). */
  forceDisconnect(userId: string, deviceId: string): void {
    const channels = this.users.get(userId);
    if (!channels) return;
    if (channels.desktop?.deviceId === deviceId) channels.desktop.sender.close();
    for (const m of channels.mobiles) {
      if (m.deviceId === deviceId) m.sender.close();
    }
  }
}

export function presenceFrame(online: boolean): string {
  return JSON.stringify({
    v: 1,
    id: crypto.randomUUID(),
    type: "presence",
    payload: { device: "desktop", online, lastSeenAt: new Date().toISOString() },
  });
}

function broadcastPresence(mobiles: Connection[], online: boolean): void {
  const frame = presenceFrame(online);
  for (const conn of mobiles) conn.sender.send(frame);
}
