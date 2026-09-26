import { Box, Button, Checkbox, Dialog, HStack, Portal, Spinner, Text, VStack } from "@chakra-ui/react";
import type { ProjectPick } from "@job-hunter/types";
import { Sparkles } from "lucide-react";
import { useEffect, useState } from "react";
import { ErrorBanner } from "./common";
import { useExperiences, useProjects, usePublishingPlan, useSavePublishingPlan, useSuggestProjectPicks } from "../lib/queries";

/**
 * Choose which projects appear on the job-site profiles. Claude suggests the
 * strongest ones with a reason each; the user adjusts and clicks Proceed.
 * Nothing is published before that.
 */
export function ProjectPicker({ open, platform, onClose, onConfirmed }: { open: boolean; platform: string | null; onClose: () => void; onConfirmed: (platform: string | null) => void }) {
  const plan = usePublishingPlan();
  const projects = useProjects();
  const experiences = useExperiences();
  const suggest = useSuggestProjectPicks();
  const save = useSavePublishingPlan();
  const [picks, setPicks] = useState<ProjectPick[]>([]);
  const [selected, setSelected] = useState<string[]>([]);

  const askClaude = () =>
    suggest.mutate(undefined, {
      onSuccess: (p) => {
        setPicks(p);
        setSelected(p.filter((x) => x.selected).map((x) => x.projectId));
      },
    });

  // On open: start from the saved choice, or ask Claude for one.
  useEffect(() => {
    if (!open || !plan.data) return;
    if (plan.data.confirmedAt) {
      setPicks(plan.data.picks);
      setSelected(plan.data.featuredProjectIds);
    } else {
      askClaude();
    }
  }, [open, plan.data?.confirmedAt]);

  const all = projects.data ?? [];
  // Claude's order first (strongest first), then anything it didn't rank.
  const ordered = [...picks.map((p) => all.find((x) => x.id === p.projectId)).filter((p) => !!p), ...all.filter((p) => !picks.some((x) => x.projectId === p.id))];
  const reason = (id: string) => picks.find((p) => p.projectId === id)?.reason;
  const employer = (id: string | null) => experiences.data?.find((e) => e.id === id)?.company;
  const toggle = (id: string, on: boolean) => setSelected((s) => (on ? [...s, id] : s.filter((x) => x !== id)));
  const proceed = () =>
    save.mutate(
      { featuredProjectIds: ordered.filter((p) => selected.includes(p.id)).map((p) => p.id), picks },
      {
        onSuccess: () => {
          onClose();
          onConfirmed(platform);
        },
      },
    );

  return (
    <Dialog.Root open={open} onOpenChange={(e) => !e.open && onClose()} placement="center" size="lg" scrollBehavior="inside">
      <Portal>
        <Dialog.Backdrop />
        <Dialog.Positioner>
          <Dialog.Content>
            <Dialog.Header>
              <Dialog.Title>{platform ? `Update your ${platform} profile` : "Projects on your profiles"}</Dialog.Title>
            </Dialog.Header>
            <Dialog.Body>
              <Text fontSize="sm" color="fg.muted" mb={4}>
                {platform
                  ? `Claude will edit your ${platform} profile in Chrome to match this app: headline, summary, skills, experience, education, career preferences and the projects you tick below. It never posts to your feed, messages anyone, or changes account settings. After this first update, changes you make here are synced to ${platform} automatically (you can turn that off).`
                  : "Only the projects you tick appear on your job-site profiles. Claude suggests the strongest ones; change them as you like."}
              </Text>
              <ErrorBanner error={suggest.error ?? save.error} />
              {suggest.isPending ? (
                <HStack py={6} justify="center" color="fg.muted">
                  <Spinner size="sm" />
                  <Text fontSize="sm">Claude is picking your strongest projects…</Text>
                </HStack>
              ) : all.length === 0 ? (
                <Text fontSize="sm" color="fg.muted">
                  You have no projects yet. Add some on the Candidate page; the profile update works without them too.
                </Text>
              ) : (
                <VStack align="stretch" gap={1}>
                  {ordered.map((p) => (
                    <Checkbox.Root key={p.id} checked={selected.includes(p.id)} onCheckedChange={(e) => toggle(p.id, !!e.checked)} alignItems="flex-start" py={2} px={2} borderRadius="md" _hover={{ bg: "bg.muted" }}>
                      <Checkbox.HiddenInput />
                      <Checkbox.Control mt={0.5} />
                      <Box>
                        <Checkbox.Label fontWeight="medium">
                          {p.name}
                          <Text as="span" fontWeight="normal" color="fg.muted">
                            {[employer(p.experienceId), p.role].filter(Boolean).length ? ` · ${[employer(p.experienceId), p.role].filter(Boolean).join(" · ")}` : ""}
                          </Text>
                        </Checkbox.Label>
                        {reason(p.id) ? (
                          <Text fontSize="xs" color="fg.muted" fontStyle="italic" mt={0.5}>
                            {reason(p.id)}
                          </Text>
                        ) : null}
                      </Box>
                    </Checkbox.Root>
                  ))}
                </VStack>
              )}
            </Dialog.Body>
            <Dialog.Footer justifyContent="space-between">
              <Button variant="ghost" size="sm" onClick={askClaude} loading={suggest.isPending} disabled={all.length === 0}>
                <Sparkles size={14} /> Ask Claude again
              </Button>
              <HStack>
                <Button variant="ghost" onClick={onClose}>
                  Cancel
                </Button>
                <Button colorPalette="brand" onClick={proceed} loading={save.isPending} disabled={suggest.isPending}>
                  {platform ? `Proceed (${selected.length} project${selected.length === 1 ? "" : "s"})` : "Save"}
                </Button>
              </HStack>
            </Dialog.Footer>
          </Dialog.Content>
        </Dialog.Positioner>
      </Portal>
    </Dialog.Root>
  );
}
