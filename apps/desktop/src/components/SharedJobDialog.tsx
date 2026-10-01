import { Box, Button, Checkbox, Dialog, HStack, Portal, Text, Textarea, VStack } from "@chakra-ui/react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { fileName } from "@job-hunter/shared";
import { ImagePlus, Sparkles, X } from "lucide-react";
import { useState } from "react";
import { ErrorBanner } from "./common";
import { useAddSharedJob } from "../lib/queries";

/**
 * A job found outside Job Hunter (a LinkedIn post, a WhatsApp forward, a PDF):
 * paste it and/or add screenshots; Claude reads it, writes the resume and
 * cover letter, and saves the email as a Gmail draft for you to send.
 */
export function SharedJobDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const add = useAddSharedJob();
  const [text, setText] = useState("");
  const [files, setFiles] = useState<string[]>([]);
  const [draftEmail, setDraftEmail] = useState(true);

  const pick = async () => {
    const chosen = await openDialog({ multiple: true, filters: [{ name: "Screenshots and PDFs", extensions: ["png", "jpg", "jpeg", "webp", "gif", "pdf"] }] });
    const list = Array.isArray(chosen) ? chosen : chosen ? [chosen] : [];
    setFiles((f) => [...f, ...list.filter((p) => !f.includes(p))].slice(0, 10));
  };
  const close = () => {
    if (add.isPending) return;
    onClose();
  };
  const submit = () =>
    add.mutate(
      { text, files, draftEmail },
      {
        onSuccess: () => {
          setText("");
          setFiles([]);
          onClose();
        },
      },
    );
  const ready = text.trim().length >= 30 || files.length > 0;

  return (
    <Dialog.Root open={open} onOpenChange={(e) => !e.open && close()} placement="center" size="lg">
      <Portal>
        <Dialog.Backdrop />
        <Dialog.Positioner>
          <Dialog.Content>
            <Dialog.Header>
              <Dialog.Title>Add a job you found</Dialog.Title>
            </Dialog.Header>
            <Dialog.Body>
              <Text fontSize="sm" color="fg.muted" mb={3}>
                Paste the job description and/or add screenshots or PDFs of it. Claude reads it, writes a tailored resume and cover letter, and creates the
                application for your review.
              </Text>
              <ErrorBanner error={add.error} title="Couldn't start" />
              <Textarea rows={8} value={text} onChange={(e) => setText(e.target.value)} placeholder="Paste the job post here (optional if you add screenshots or PDFs)" />
              <HStack mt={3} gap={2} wrap="wrap">
                <Button size="sm" variant="outline" onClick={() => void pick()} disabled={files.length >= 10}>
                  <ImagePlus size={14} /> Add screenshots or PDFs
                </Button>
                {files.map((f) => (
                  <HStack key={f} gap={1} px={2} py={1} borderWidth="1px" borderColor="border.muted" borderRadius="md" fontSize="xs">
                    <Text truncate maxW="200px">
                      {fileName(f)}
                    </Text>
                    <Box as="button" aria-label={`Remove ${fileName(f)}`} onClick={() => setFiles((list) => list.filter((x) => x !== f))}>
                      <X size={12} />
                    </Box>
                  </HStack>
                ))}
              </HStack>
              <VStack align="stretch" mt={4}>
                <Checkbox.Root checked={draftEmail} onCheckedChange={(e) => setDraftEmail(!!e.checked)} size="sm">
                  <Checkbox.HiddenInput />
                  <Checkbox.Control />
                  <Checkbox.Label>Save the email as a Gmail draft with the resume attached (you review and send it)</Checkbox.Label>
                </Checkbox.Root>
              </VStack>
            </Dialog.Body>
            <Dialog.Footer>
              <Button variant="ghost" onClick={close} disabled={add.isPending}>
                Cancel
              </Button>
              <Button colorPalette="brand" onClick={submit} loading={add.isPending} disabled={!ready}>
                <Sparkles size={14} /> Prepare application
              </Button>
            </Dialog.Footer>
          </Dialog.Content>
        </Dialog.Positioner>
      </Portal>
    </Dialog.Root>
  );
}
