import { Box, Button, Heading, HStack, Input, SimpleGrid, Textarea, VStack } from "@chakra-ui/react";
import type { ResumeDocument, ResumeExperience, ResumeProject, ResumeSkillSection } from "@job-hunter/types";
import { Plus, Trash2 } from "lucide-react";
import { useState } from "react";
import { LinesInput, TagInput } from "./common";

/** Structured editor for a generated resume. Saves re-render DOCX/PDF. */
export function ResumeEditor({ value, onSave, onCancel, saving }: { value: ResumeDocument; onSave: (doc: ResumeDocument) => void; onCancel: () => void; saving: boolean }) {
  const [doc, setDoc] = useState<ResumeDocument>(structuredClone(value));
  const set = <K extends keyof ResumeDocument>(k: K, v: ResumeDocument[K]) => setDoc((d) => ({ ...d, [k]: v }));

  return (
    <VStack align="stretch" gap={5}>
      <Box>
        <Heading size="xs" mb={2}>
          Headline &amp; summary
        </Heading>
        <Input size="sm" mb={2} value={doc.headline} onChange={(e) => set("headline", e.target.value)} placeholder="Headline" />
        <Textarea size="sm" rows={4} value={doc.summary} onChange={(e) => set("summary", e.target.value)} placeholder="Summary" />
      </Box>
      <Box>
        <HStack justify="space-between" mb={2}>
          <Heading size="xs">Skills</Heading>
          <Button size="xs" variant="ghost" onClick={() => set("skills", [...doc.skills, { name: "New section", items: [] }])}>
            <Plus size={12} /> Section
          </Button>
        </HStack>
        <VStack align="stretch" gap={2}>
          {doc.skills.map((s, i) => (
            <HStack key={i} align="flex-start">
              <Input size="sm" w="160px" value={s.name} onChange={(e) => set("skills", doc.skills.map((x, j) => (j === i ? { ...x, name: e.target.value } : x)))} />
              <Box flex="1">
                <TagInput value={s.items} onChange={(items) => set("skills", doc.skills.map((x, j) => (j === i ? { ...x, items } : x)))} />
              </Box>
              <Button size="xs" variant="ghost" colorPalette="red" onClick={() => set("skills", doc.skills.filter((_, j) => j !== i))}>
                <Trash2 size={12} />
              </Button>
            </HStack>
          ))}
        </VStack>
      </Box>
      <Box>
        <Heading size="xs" mb={2}>
          Experience
        </Heading>
        <VStack align="stretch" gap={3}>
          {doc.experience.map((e, i) => (
            <ExperienceRow key={i} value={e} onChange={(v) => set("experience", doc.experience.map((x, j) => (j === i ? v : x)))} onRemove={() => set("experience", doc.experience.filter((_, j) => j !== i))} />
          ))}
        </VStack>
      </Box>
      <Box>
        <HStack justify="space-between" mb={2}>
          <Heading size="xs">Projects</Heading>
        </HStack>
        <VStack align="stretch" gap={3}>
          {doc.projects.map((p, i) => (
            <ProjectRow key={i} value={p} onChange={(v) => set("projects", doc.projects.map((x, j) => (j === i ? v : x)))} onRemove={() => set("projects", doc.projects.filter((_, j) => j !== i))} />
          ))}
        </VStack>
      </Box>
      <SimpleGrid columns={2} gap={3}>
        <Box>
          <Heading size="xs" mb={2}>
            Certifications
          </Heading>
          <LinesInput value={doc.certifications} onChange={(v) => set("certifications", v)} rows={3} />
        </Box>
        <Box>
          <Heading size="xs" mb={2}>
            Achievements
          </Heading>
          <LinesInput value={doc.achievements} onChange={(v) => set("achievements", v)} rows={3} />
        </Box>
      </SimpleGrid>
      <HStack justify="flex-end">
        <Button variant="ghost" size="sm" onClick={onCancel}>
          Cancel
        </Button>
        <Button colorPalette="brand" size="sm" loading={saving} onClick={() => onSave(doc)}>
          Save &amp; re-render documents
        </Button>
      </HStack>
    </VStack>
  );
}

function ExperienceRow({ value, onChange, onRemove }: { value: ResumeExperience; onChange: (v: ResumeExperience) => void; onRemove: () => void }) {
  return (
    <Box p={3} borderWidth="1px" borderColor="border.muted" borderRadius="md">
      <SimpleGrid columns={{ base: 2, md: 4 }} gap={2} mb={2}>
        <Input size="sm" value={value.role} placeholder="Role" onChange={(e) => onChange({ ...value, role: e.target.value })} />
        <Input size="sm" value={value.company} placeholder="Company" onChange={(e) => onChange({ ...value, company: e.target.value })} />
        <Input size="sm" value={value.startDate} placeholder="Start" onChange={(e) => onChange({ ...value, startDate: e.target.value })} />
        <Input size="sm" value={value.endDate} placeholder="End (blank = present)" onChange={(e) => onChange({ ...value, endDate: e.target.value })} />
      </SimpleGrid>
      <LinesInput value={value.bullets} onChange={(bullets) => onChange({ ...value, bullets })} rows={4} placeholder="One bullet per line" />
      <HStack justify="flex-end" mt={1}>
        <Button size="xs" variant="ghost" colorPalette="red" onClick={onRemove}>
          <Trash2 size={12} /> Remove
        </Button>
      </HStack>
    </Box>
  );
}

function ProjectRow({ value, onChange, onRemove }: { value: ResumeProject; onChange: (v: ResumeProject) => void; onRemove: () => void }) {
  return (
    <Box p={3} borderWidth="1px" borderColor="border.muted" borderRadius="md">
      <HStack mb={2}>
        <Input size="sm" value={value.name} placeholder="Project" onChange={(e) => onChange({ ...value, name: e.target.value })} />
        <Box flex="1">
          <TagInput value={value.technologies} onChange={(technologies) => onChange({ ...value, technologies })} placeholder="Technologies" />
        </Box>
      </HStack>
      <Textarea size="sm" rows={2} value={value.description} placeholder="Description" onChange={(e) => onChange({ ...value, description: e.target.value })} />
      <HStack justify="flex-end" mt={1}>
        <Button size="xs" variant="ghost" colorPalette="red" onClick={onRemove}>
          <Trash2 size={12} /> Remove
        </Button>
      </HStack>
    </Box>
  );
}

export type { ResumeSkillSection };
