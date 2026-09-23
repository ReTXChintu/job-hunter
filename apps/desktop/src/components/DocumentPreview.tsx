import { Box, Button, HStack, Spinner, Text } from "@chakra-ui/react";
import type { CoverLetter, Resume } from "@job-hunter/types";
import { fileName } from "@job-hunter/shared";
import { ExternalLink } from "lucide-react";
import { useOpenPath, useTextFile } from "../lib/queries";
import { useTheme } from "next-themes";

/** Renders the generated HTML resume inside a sandboxed iframe. */
export function HtmlPreview({ path, height = "720px" }: { path: string | null | undefined; height?: string }) {
  const file = useTextFile(path);
  const { resolvedTheme } = useTheme();
  if (!path)
    return (
      <Text fontSize="sm" color="fg.subtle">
        No preview available.
      </Text>
    );
  if (file.isLoading) return <Spinner size="sm" />;
  if (file.error)
    return (
      <Text fontSize="sm" color="red.fg">
        Could not load preview.
      </Text>
    );
  const html = (file.data ?? "").replace("</head>", `<style>html{background:${resolvedTheme === "dark" ? "#2a2c33" : "#e5e7eb"};padding:16px 0}.page{background:#fff;box-shadow:0 1px 6px rgba(0,0,0,.25)}</style></head>`);
  return (
    <Box borderWidth="1px" borderColor="border.muted" borderRadius="md" overflow="hidden" bg="bg.muted">
      <iframe title="Document preview" sandbox="" srcDoc={html} style={{ width: "100%", height, border: 0, display: "block" }} />
    </Box>
  );
}

export function FileLinks({ resume, coverLetter }: { resume?: Resume | null; coverLetter?: CoverLetter | null }) {
  const open = useOpenPath();
  const links: { label: string; path: string | null | undefined }[] = [
    { label: "Resume PDF", path: resume?.pdfPath },
    { label: "Resume DOCX", path: resume?.docxPath },
    { label: "Cover letter PDF", path: coverLetter?.pdfPath },
    { label: "Cover letter DOCX", path: coverLetter?.docxPath },
  ];
  const available = links.filter((l) => !!l.path);
  if (!available.length) return null;
  return (
    <HStack gap={2} wrap="wrap">
      {available.map((l) => (
        <Button key={l.label} size="xs" variant="outline" onClick={() => open.mutate(l.path!)} title={fileName(l.path)}>
          <ExternalLink size={12} /> {l.label}
        </Button>
      ))}
    </HStack>
  );
}
