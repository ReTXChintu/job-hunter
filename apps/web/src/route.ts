import { useEffect, useState } from "react";

/**
 * Hash-based routes (`#/applications/<id>`), so the backend can serve the
 * app as plain static files with no server-side fallback route.
 */
export type Route =
  | { page: "overview" }
  | { page: "applications"; group?: string }
  | { page: "application"; id: string }
  | { page: "jobs" };

export function parseRoute(hash: string): Route {
  const [path = "", query = ""] = hash.replace(/^#\/?/, "").split("?");
  const parts = path.split("/").filter(Boolean);
  if (parts[0] === "applications" && parts[1]) return { page: "application", id: decodeURIComponent(parts[1]) };
  if (parts[0] === "applications") {
    const group = new URLSearchParams(query).get("group") ?? undefined;
    return group ? { page: "applications", group } : { page: "applications" };
  }
  if (parts[0] === "jobs") return { page: "jobs" };
  return { page: "overview" };
}

export function href(route: Route): string {
  switch (route.page) {
    case "overview":
      return "#/";
    case "applications":
      return route.group ? `#/applications?group=${encodeURIComponent(route.group)}` : "#/applications";
    case "application":
      return `#/applications/${encodeURIComponent(route.id)}`;
    case "jobs":
      return "#/jobs";
  }
}

export function useRoute(): Route {
  const [route, setRoute] = useState(() => parseRoute(window.location.hash));
  useEffect(() => {
    const onChange = () => {
      setRoute(parseRoute(window.location.hash));
      window.scrollTo(0, 0);
    };
    window.addEventListener("hashchange", onChange);
    return () => window.removeEventListener("hashchange", onChange);
  }, []);
  return route;
}
