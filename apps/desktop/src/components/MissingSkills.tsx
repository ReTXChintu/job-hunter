import { Button, Flex, HStack, NativeSelect, Text, VStack } from "@chakra-ui/react";
import { guessSkillGroup, SKILL_GROUPS, type SkillGroupKey } from "@job-hunter/shared";
import { Check } from "lucide-react";
import { useEffect, useState } from "react";
import { ErrorBanner } from "./common";
import { useMarkSkillsKnown } from "../lib/queries";

/**
 * Skills (or keywords) a job asks for that your profile doesn't list. Click the
 * ones you actually know to add them to your skills: every analysis then
 * counts them as matched, and the next resume can use them.
 */
export function MissingSkills({ skills, max = 20, onMarked }: { skills: string[]; max?: number; onMarked?: (skills: string[]) => void }) {
  const mark = useMarkSkillsKnown();
  const [selected, setSelected] = useState<string[]>([]);
  const [group, setGroup] = useState<SkillGroupKey>("other");

  // Drop selections that are no longer missing (e.g. after marking them).
  useEffect(() => setSelected((sel) => sel.filter((s) => skills.includes(s))), [skills]);

  if (!skills.length)
    return (
      <Text fontSize="sm" color="fg.subtle">
        None
      </Text>
    );

  const toggle = (skill: string) => {
    const next = selected.includes(skill) ? selected.filter((s) => s !== skill) : [...selected, skill];
    if (!selected.length && next.length) setGroup(guessSkillGroup(skill));
    setSelected(next);
  };
  const submit = () =>
    mark.mutate(
      { skills: selected, group },
      {
        onSuccess: () => {
          onMarked?.(selected);
          setSelected([]);
        },
      },
    );

  return (
    <VStack align="stretch" gap={2}>
      <Flex wrap="wrap" gap={1.5}>
        {skills.slice(0, max).map((skill) => {
          const on = selected.includes(skill);
          return (
            <Button
              key={skill}
              size="2xs"
              borderRadius="full"
              px={2.5}
              colorPalette="orange"
              variant={on ? "solid" : "subtle"}
              fontWeight="medium"
              title={on ? "Selected: you know this" : "I know this"}
              onClick={() => toggle(skill)}
            >
              {on ? <Check size={11} /> : null}
              {skill}
            </Button>
          );
        })}
        {skills.length > max ? (
          <Text fontSize="xs" color="fg.muted" alignSelf="center">
            +{skills.length - max} more
          </Text>
        ) : null}
      </Flex>
      <ErrorBanner error={mark.error} title="Couldn't update your skills" />
      {selected.length ? (
        <HStack gap={2} wrap="wrap">
          <Text fontSize="xs" color="fg.muted">
            Add {selected.length === 1 ? selected[0] : `${selected.length} skills`} to your skills under
          </Text>
          <NativeSelect.Root size="xs" width="auto">
            <NativeSelect.Field value={group} onChange={(e) => setGroup(e.target.value as SkillGroupKey)} textTransform="capitalize">
              {SKILL_GROUPS.map((g) => (
                <option key={g} value={g}>
                  {g}
                </option>
              ))}
            </NativeSelect.Field>
            <NativeSelect.Indicator />
          </NativeSelect.Root>
          <Button size="xs" colorPalette="brand" onClick={submit} loading={mark.isPending}>
            <Check size={12} /> I know {selected.length === 1 ? "this" : "these"}
          </Button>
        </HStack>
      ) : (
        <Text fontSize="xs" color="fg.subtle">
          Know one of these? Click it to add it to your skills.
        </Text>
      )}
    </VStack>
  );
}
