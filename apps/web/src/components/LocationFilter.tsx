import { Flex, IconButton, Input } from "@chakra-ui/react";
import type { LocationOption } from "@job-hunter/shared";
import { X } from "lucide-react";
import { useId } from "react";

/**
 * Filter a list by place: type any part of a location or pick one of the
 * places already in the data, with how many jobs each. Looks only through
 * what's stored, never the web.
 */
export function LocationFilter({ value, onChange, options }: { value: string; onChange: (value: string) => void; options: LocationOption[] }) {
  const listId = useId();
  return (
    <Flex gap={1} align="center">
      <Input size="sm" maxW={{ md: "200px" }} list={listId} placeholder="Any location" value={value} onChange={(e) => onChange(e.target.value)} aria-label="Location" />
      <datalist id={listId}>
        {options.slice(0, 60).map((o) => (
          <option key={o.label} value={o.label}>
            {o.count === 1 ? "1 job" : `${o.count} jobs`}
          </option>
        ))}
      </datalist>
      {value ? (
        <IconButton size="xs" variant="ghost" aria-label="Clear location" onClick={() => onChange("")}>
          <X size={12} />
        </IconButton>
      ) : null}
    </Flex>
  );
}
