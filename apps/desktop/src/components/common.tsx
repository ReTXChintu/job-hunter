import { Alert, Box, Button, Collapsible, Dialog, Flex, Heading, HStack, Input, Portal, Tag, Text, Textarea } from "@chakra-ui/react";
import { splitList } from "@job-hunter/shared";
import { X } from "lucide-react";
import { useState, type KeyboardEvent, type ReactNode } from "react";
import { errorDetails, errorMessage } from "../lib/tauri";

export function PageHeader({ title, subtitle, actions }: { title: string; subtitle?: ReactNode; actions?: ReactNode }) {
  return (
    <Flex justify="space-between" align="flex-start" gap={4} mb={6}>
      <Box>
        <Heading size="xl">{title}</Heading>
        {subtitle ? (
          <Text color="fg.muted" mt={1}>
            {subtitle}
          </Text>
        ) : null}
      </Box>
      {actions ? <HStack gap={2}>{actions}</HStack> : null}
    </Flex>
  );
}

export function ErrorBanner({ error, title = "Something went wrong", onRetry }: { error: unknown; title?: string; onRetry?: () => void }) {
  const [open, setOpen] = useState(false);
  if (!error) return null;
  const details = errorDetails(error);
  return (
    <Alert.Root status="error" variant="subtle" mb={4} alignItems="flex-start">
      <Alert.Indicator />
      <Alert.Content>
        <Alert.Title>{title}</Alert.Title>
        <Alert.Description>{errorMessage(error)}</Alert.Description>
        <HStack mt={2} gap={2}>
          {onRetry ? (
            <Button size="xs" variant="subtle" onClick={onRetry}>
              Retry
            </Button>
          ) : null}
          {details ? (
            <Button size="xs" variant="ghost" onClick={() => setOpen((o) => !o)}>
              {open ? "Hide" : "Show"} developer details
            </Button>
          ) : null}
        </HStack>
        {details ? (
          <Collapsible.Root open={open}>
            <Collapsible.Content>
              <Box as="pre" mt={2} p={2} fontSize="xs" fontFamily="mono" bg="bg.muted" borderRadius="sm" whiteSpace="pre-wrap" className="selectable" maxH="240px" overflow="auto">
                {details}
              </Box>
            </Collapsible.Content>
          </Collapsible.Root>
        ) : null}
      </Alert.Content>
    </Alert.Root>
  );
}

export function InfoBanner({ status = "info", title, children }: { status?: "info" | "warning" | "success" | "error"; title?: string; children: ReactNode }) {
  return (
    <Alert.Root status={status} variant="subtle" mb={4}>
      <Alert.Indicator />
      <Alert.Content>
        {title ? <Alert.Title>{title}</Alert.Title> : null}
        <Alert.Description>{children}</Alert.Description>
      </Alert.Content>
    </Alert.Root>
  );
}

export function Panel({ children, title, action, p = 5, ...rest }: { children: ReactNode; title?: string; action?: ReactNode; p?: number; [k: string]: unknown }) {
  return (
    <Box borderWidth="1px" borderColor="border.muted" borderRadius="md" bg="bg.panel" p={p} {...rest}>
      {title ? (
        <Flex justify="space-between" align="center" mb={3}>
          <Heading size="sm">{title}</Heading>
          {action}
        </Flex>
      ) : null}
      {children}
    </Box>
  );
}

export function ConfirmDialog({ open, onOpenChange, title, description, confirmLabel = "Confirm", destructive = false, onConfirm, loading = false, children }: { open: boolean; onOpenChange: (open: boolean) => void; title: string; description?: ReactNode; confirmLabel?: string; destructive?: boolean; onConfirm: () => void; loading?: boolean; children?: ReactNode }) {
  return (
    <Dialog.Root open={open} onOpenChange={(e) => onOpenChange(e.open)} placement="center">
      <Portal>
        <Dialog.Backdrop />
        <Dialog.Positioner>
          <Dialog.Content>
            <Dialog.Header>
              <Dialog.Title>{title}</Dialog.Title>
            </Dialog.Header>
            <Dialog.Body>
              {description ? <Text color="fg.muted">{description}</Text> : null}
              {children}
            </Dialog.Body>
            <Dialog.Footer>
              <Button variant="ghost" onClick={() => onOpenChange(false)}>
                Cancel
              </Button>
              <Button colorPalette={destructive ? "red" : "brand"} onClick={onConfirm} loading={loading}>
                {confirmLabel}
              </Button>
            </Dialog.Footer>
          </Dialog.Content>
        </Dialog.Positioner>
      </Portal>
    </Dialog.Root>
  );
}

/** Comma/enter separated tag editor for string lists. */
export function TagInput({ value, onChange, placeholder }: { value: string[]; onChange: (v: string[]) => void; placeholder?: string }) {
  const [draft, setDraft] = useState("");
  const commit = () => {
    const items = splitList(draft);
    if (items.length) onChange([...value, ...items.filter((i) => !value.includes(i))]);
    setDraft("");
  };
  const onKey = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter" || e.key === ",") {
      e.preventDefault();
      commit();
    } else if (e.key === "Backspace" && !draft && value.length) {
      onChange(value.slice(0, -1));
    }
  };
  return (
    <Box borderWidth="1px" borderColor="border" borderRadius="md" px={2} py={1.5} _focusWithin={{ borderColor: "brand.focusRing", boxShadow: "0 0 0 1px var(--chakra-colors-brand-focusRing)" }}>
      <Flex wrap="wrap" gap={1.5} align="center">
        {value.map((item) => (
          <Tag.Root key={item} size="md" variant="subtle">
            <Tag.Label>{item}</Tag.Label>
            <Tag.EndElement>
              <Tag.CloseTrigger onClick={() => onChange(value.filter((v) => v !== item))}>
                <X size={12} />
              </Tag.CloseTrigger>
            </Tag.EndElement>
          </Tag.Root>
        ))}
        <Input variant="flushed" size="sm" border="none" _focus={{ boxShadow: "none" }} flex="1" minW="140px" value={draft} placeholder={placeholder ?? "Type and press Enter"} onChange={(e) => setDraft(e.target.value)} onKeyDown={onKey} onBlur={commit} />
      </Flex>
    </Box>
  );
}

export function LinesInput({ value, onChange, placeholder, rows = 4 }: { value: string[]; onChange: (v: string[]) => void; placeholder?: string; rows?: number }) {
  return <Textarea rows={rows} value={value.join("\n")} placeholder={placeholder ?? "One item per line"} onChange={(e) => onChange(e.target.value.split("\n").map((s) => s.replace(/^[-•*]\s*/, "")).filter((_, i, arr) => i < arr.length - 1 || _.length > 0))} />;
}

export function Stat({ label, value, tone = "fg" }: { label: string; value: ReactNode; tone?: string }) {
  return (
    <Box px={4} py={3} borderWidth="1px" borderColor="border.muted" borderRadius="md" bg="bg.panel" minW="130px">
      <Text fontSize="xs" color="fg.muted" fontWeight="semibold" textTransform="uppercase" letterSpacing="wide">
        {label}
      </Text>
      <Text fontSize="2xl" fontWeight="bold" color={tone} lineHeight="1.2" mt={1} fontVariantNumeric="tabular-nums">
        {value}
      </Text>
    </Box>
  );
}

export function Chips({ items, palette = "gray", max = 12 }: { items: string[]; palette?: string; max?: number }) {
  if (!items.length)
    return (
      <Text fontSize="sm" color="fg.subtle">
        None
      </Text>
    );
  return (
    <Flex wrap="wrap" gap={1.5}>
      {items.slice(0, max).map((i) => (
        <Tag.Root key={i} size="sm" variant="subtle" colorPalette={palette}>
          <Tag.Label>{i}</Tag.Label>
        </Tag.Root>
      ))}
      {items.length > max ? (
        <Text fontSize="xs" color="fg.muted" alignSelf="center">
          +{items.length - max} more
        </Text>
      ) : null}
    </Flex>
  );
}

export function BulletList({ items }: { items: string[] }) {
  if (!items.length)
    return (
      <Text fontSize="sm" color="fg.subtle">
        None listed
      </Text>
    );
  return (
    <Box as="ul" pl={5} fontSize="sm" className="selectable">
      {items.map((i, idx) => (
        <Box as="li" key={idx} mb={1}>
          {i}
        </Box>
      ))}
    </Box>
  );
}
