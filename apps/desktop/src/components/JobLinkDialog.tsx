import { Button, Checkbox, Dialog, Field, Input, Portal, Text, VStack } from "@chakra-ui/react";
import { Play } from "lucide-react";
import { useState } from "react";
import { ErrorBanner } from "./common";
import { useApplyFromLink } from "../lib/queries";

/**
 * Paste a job posting's link from any job site and start: the agent opens
 * it, reads the posting, prepares a tailored resume and cover letter, and
 * (when ticked) approves and applies as soon as they're ready.
 */
export function JobLinkDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const start = useApplyFromLink();
  const [url, setUrl] = useState("");
  const [applyNow, setApplyNow] = useState(true);
  const valid = /^https?:\/\/\S+\.\S+/i.test(url.trim());
  const close = () => {
    if (!start.isPending) onClose();
  };
  const submit = () =>
    start.mutate(
      { url: url.trim(), applyNow },
      {
        onSuccess: () => {
          setUrl("");
          onClose();
        },
      },
    );

  return (
    <Dialog.Root open={open} onOpenChange={(e) => !e.open && close()} placement="center" size="md">
      <Portal>
        <Dialog.Backdrop />
        <Dialog.Positioner>
          <Dialog.Content>
            <Dialog.Header>
              <Dialog.Title>Apply from a link</Dialog.Title>
            </Dialog.Header>
            <Dialog.Body>
              <ErrorBanner error={start.error} title="Couldn't start" />
              <Field.Root invalid={url.trim().length > 0 && !valid}>
                <Field.Label>Job posting link</Field.Label>
                <Input
                  autoFocus
                  placeholder="https://www.linkedin.com/jobs/view/…"
                  value={url}
                  onChange={(e) => setUrl(e.target.value)}
                  onKeyDown={(e) => e.key === "Enter" && valid && submit()}
                />
                <Field.HelperText>From LinkedIn, Naukri, Indeed, Wellfound, Cutshort or any other job site or careers page.</Field.HelperText>
              </Field.Root>
              <VStack align="stretch" mt={4} gap={1}>
                <Checkbox.Root checked={applyNow} onCheckedChange={(e) => setApplyNow(!!e.checked)} size="sm">
                  <Checkbox.HiddenInput />
                  <Checkbox.Control />
                  <Checkbox.Label>Apply right away with the tailored resume (counts as my approval)</Checkbox.Label>
                </Checkbox.Root>
                <Text fontSize="xs" color="fg.muted" pl={6}>
                  {applyNow
                    ? "It still stops to ask you anything the form needs that your profile doesn't answer, and never claims success without the site's confirmation."
                    : "It prepares the application and waits for you to review and approve it."}
                </Text>
              </VStack>
            </Dialog.Body>
            <Dialog.Footer>
              <Button variant="ghost" onClick={close} disabled={start.isPending}>
                Cancel
              </Button>
              <Button colorPalette="brand" onClick={submit} loading={start.isPending} disabled={!valid}>
                <Play size={14} /> Start
              </Button>
            </Dialog.Footer>
          </Dialog.Content>
        </Dialog.Positioner>
      </Portal>
    </Dialog.Root>
  );
}
