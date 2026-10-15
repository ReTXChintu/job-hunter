import { Button, HStack, IconButton, Input, Switch, Text, VStack } from "@chakra-ui/react";
import type { TargetCompany } from "@job-hunter/types";
import { Plus, Trash2 } from "lucide-react";

/**
 * The companies whose own careers sites the "Company careers" source
 * searches. Each one switched on is a separate Claude run per hunt.
 */
export function CompaniesEditor({ value, onChange }: { value: TargetCompany[]; onChange: (next: TargetCompany[]) => void }) {
  const set = (i: number, patch: Partial<TargetCompany>) => onChange(value.map((c, j) => (j === i ? { ...c, ...patch } : c)));
  const on = value.filter((c) => c.enabled).length;
  return (
    <VStack align="stretch" gap={2}>
      <Text fontSize="sm" color="fg.muted">
        {on} of {value.length} on. Each one switched on is searched on its own careers site in every hunt (one Claude run each), so keep the list to the companies you'd really join.
      </Text>
      {value.map((c, i) => (
        <HStack key={i} gap={2}>
          <Switch.Root size="sm" checked={c.enabled} onCheckedChange={(e) => set(i, { enabled: !!e.checked })} aria-label={`Search ${c.name || "this company"}`}>
            <Switch.HiddenInput />
            <Switch.Control />
          </Switch.Root>
          <Input size="sm" w="180px" value={c.name} placeholder="Company" onChange={(e) => set(i, { name: e.target.value })} />
          <Input size="sm" flex="1" value={c.careersUrl} placeholder="https://careers.example.com/jobs" onChange={(e) => set(i, { careersUrl: e.target.value })} />
          <IconButton size="sm" variant="ghost" aria-label={`Remove ${c.name || "company"}`} onClick={() => onChange(value.filter((_, j) => j !== i))}>
            <Trash2 size={14} />
          </IconButton>
        </HStack>
      ))}
      <HStack>
        <Button size="sm" variant="outline" onClick={() => onChange([...value, { name: "", careersUrl: "", enabled: true }])}>
          <Plus size={14} /> Add a company
        </Button>
      </HStack>
    </VStack>
  );
}
