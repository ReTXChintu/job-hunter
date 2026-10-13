import { HStack, IconButton, Input } from "@chakra-ui/react";
import type { LocationOption } from "@job-hunter/shared";
import { MapPin, X } from "lucide-react";
import { useId } from "react";

/**
 * Filter a list by place. Type any part of a location ("mumbai" matches
 * "Mumbai, Maharashtra") or pick one of the places already in the
 * database, listed with how many jobs each. Searches only what's stored,
 * never the web.
 */
export function LocationFilter({ value, onChange, options }: { value: string; onChange: (value: string) => void; options: LocationOption[] }) {
  const listId = useId();
  return (
    <HStack gap={1}>
      <MapPin size={14} style={{ opacity: 0.6, flexShrink: 0 }} />
      <Input size="sm" maxW="190px" list={listId} placeholder="Any location" value={value} onChange={(e) => onChange(e.target.value)} title="Only looks through the jobs already found" />
      <datalist id={listId}>
        {options.slice(0, 60).map((o) => (
          <option key={o.label} value={o.label}>
            {o.count === 1 ? "1 job" : `${o.count} jobs`}
          </option>
        ))}
      </datalist>
      {value ? <IconButton size="xs" variant="ghost" aria-label="Clear location" onClick={() => onChange("")}><X size={12} /></IconButton> : null}
    </HStack>
  );
}
