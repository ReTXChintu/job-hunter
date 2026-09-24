import { describe, expect, it } from "vitest";

import type { FrameSender } from "../src/hub.js";
import { Hub } from "../src/hub.js";

function fakeSender(): { sender: FrameSender; sent: string[]; closed: boolean } {
  const sent: string[] = [];
  let closed = false;
  return {
    sender: {
      send: (text) => sent.push(text),
      close: () => {
        closed = true;
      },
    },
    sent,
    get closed() {
      return closed;
    },
  };
}

describe("Hub", () => {
  it("tells mobiles about desktop presence on connect and disconnect", () => {
    const hub = new Hub();
    const mobile = fakeSender();
    hub.register("u1", "mobile-1", "mobile", mobile.sender);
    expect(hub.isDesktopOnline("u1")).toBe(false);

    const desktop = fakeSender();
    const outcome = hub.register("u1", "desktop-1", "desktop", desktop.sender);
    expect(outcome.desktopOnline).toBe(true);
    expect(mobile.sent).toHaveLength(1);
    expect(mobile.sent[0]).toContain('"online":true');
    expect(hub.isDesktopOnline("u1")).toBe(true);

    hub.unregister("u1", "desktop-1", "desktop");
    expect(mobile.sent).toHaveLength(2);
    expect(mobile.sent[1]).toContain('"online":false');
    expect(hub.isDesktopOnline("u1")).toBe(false);
  });

  it("routes a request to the desktop and fans replies out to mobiles", () => {
    const hub = new Hub();
    const desktop = fakeSender();
    hub.register("u1", "desktop-1", "desktop", desktop.sender);
    const mobile = fakeSender();
    hub.register("u1", "mobile-1", "mobile", mobile.sender);

    expect(hub.routeToDesktop("u1", "hello")).toBe(true);
    expect(desktop.sent).toEqual(["hello"]);

    hub.routeToMobiles("u1", "reply");
    expect(mobile.sent).toEqual(["reply"]);
  });

  it("fails cleanly routing to a desktop that isn't connected", () => {
    const hub = new Hub();
    expect(hub.routeToDesktop("no-such-user", "hi")).toBe(false);
  });

  it("closes and replaces a previous desktop connection", () => {
    const hub = new Hub();
    const first = fakeSender();
    hub.register("u1", "desktop-1", "desktop", first.sender);
    const second = fakeSender();
    hub.register("u1", "desktop-2", "desktop", second.sender);
    expect(first.closed).toBe(true);
  });

  it("force-disconnects only the named device", () => {
    const hub = new Hub();
    const mobile = fakeSender();
    hub.register("u1", "mobile-1", "mobile", mobile.sender);
    hub.forceDisconnect("u1", "mobile-1");
    expect(mobile.closed).toBe(true);
  });

  it("reports device-level online status", () => {
    const hub = new Hub();
    expect(hub.isOnline("u1", "mobile-1")).toBe(false);
    const mobile = fakeSender();
    hub.register("u1", "mobile-1", "mobile", mobile.sender);
    expect(hub.isOnline("u1", "mobile-1")).toBe(true);
    hub.unregister("u1", "mobile-1", "mobile");
    expect(hub.isOnline("u1", "mobile-1")).toBe(false);
  });
});
