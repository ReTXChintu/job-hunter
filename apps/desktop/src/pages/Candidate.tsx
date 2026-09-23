import { Box, Button, Field, Heading, HStack, Input, NativeSelect, SimpleGrid, Spinner, Tabs, Text, Textarea, VStack } from "@chakra-ui/react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { fileName, formatDateTime, profileIssues } from "@job-hunter/shared";
import type { CandidateProfile, Certification, Education, EmploymentType, Experience, Project, RelocationPreference, RemotePreference } from "@job-hunter/types";
import { EmptyState } from "@job-hunter/ui";
import { FileUp, Plus, Save, Sparkles, Trash2 } from "lucide-react";
import { useEffect, useState } from "react";
import { ErrorBanner, InfoBanner, LinesInput, PageHeader, Panel, TagInput } from "../components/common";
import { useDeleteExperience, useDeleteProject, useExperiences, useImportMasterResume, useMasterResumeText, useParseMasterResume, useProfile, useProjects, useSaveExperience, useSaveProfile, useSaveProject } from "../lib/queries";

const EMPLOYMENT: EmploymentType[] = ["FULL_TIME", "PART_TIME", "CONTRACT", "FREELANCE", "INTERNSHIP"];

export function CandidatePage() {
  const profileQ = useProfile();
  const save = useSaveProfile();
  const [draft, setDraft] = useState<CandidateProfile | null>(null);
  const [dirty, setDirty] = useState(false);

  useEffect(() => {
    if (profileQ.data && !dirty) setDraft(structuredClone(profileQ.data));
  }, [profileQ.data, dirty]);

  if (!draft) return profileQ.error ? <ErrorBanner error={profileQ.error} onRetry={() => profileQ.refetch()} /> : <Spinner />;
  const update = (fn: (p: CandidateProfile) => void) => {
    setDraft((d) => {
      if (!d) return d;
      const next = structuredClone(d);
      fn(next);
      return next;
    });
    setDirty(true);
  };
  const issues = profileIssues(draft);

  return (
    <Box>
      <PageHeader
        title="Candidate"
        subtitle="Everything the agent is allowed to know about you. It will never claim anything that is not on this page."
        actions={
          <Button colorPalette="brand" onClick={() => save.mutate(draft, { onSuccess: () => setDirty(false) })} loading={save.isPending} disabled={!dirty}>
            <Save size={16} /> Save profile
          </Button>
        }
      />
      <ErrorBanner error={save.error} />
      {issues.length ? (
        <InfoBanner status="warning" title="Before a job hunt can start">
          {issues.join(" · ")}
        </InfoBanner>
      ) : null}
      <Tabs.Root defaultValue="personal" variant="line" size="sm" lazyMount unmountOnExit>
        <Tabs.List mb={4}>
          <Tabs.Trigger value="personal">Personal</Tabs.Trigger>
          <Tabs.Trigger value="preferences">Preferences</Tabs.Trigger>
          <Tabs.Trigger value="skills">Skills</Tabs.Trigger>
          <Tabs.Trigger value="experience">Experience</Tabs.Trigger>
          <Tabs.Trigger value="projects">Projects</Tabs.Trigger>
          <Tabs.Trigger value="education">Education &amp; more</Tabs.Trigger>
          <Tabs.Trigger value="resume">Master resume</Tabs.Trigger>
        </Tabs.List>

        <Tabs.Content value="personal">
          <Panel>
            <SimpleGrid columns={{ base: 1, md: 2 }} gap={4}>
              {(
                [
                  ["name", "Name"],
                  ["email", "Email"],
                  ["phone", "Phone"],
                  ["location", "Location"],
                  ["currentTitle", "Current title"],
                  ["linkedin", "LinkedIn"],
                  ["github", "GitHub"],
                  ["portfolio", "Portfolio"],
                ] as const
              ).map(([key, label]) => (
                <Field.Root key={key}>
                  <Field.Label>{label}</Field.Label>
                  <Input value={draft.personal[key]} onChange={(e) => update((p) => { p.personal[key] = e.target.value; })} />
                </Field.Root>
              ))}
            </SimpleGrid>
            <Field.Root mt={4}>
              <Field.Label>Professional summary (optional)</Field.Label>
              <Textarea rows={3} value={draft.summary} onChange={(e) => update((p) => { p.summary = e.target.value; })} />
            </Field.Root>
          </Panel>
        </Tabs.Content>

        <Tabs.Content value="preferences">
          <Panel>
            <SimpleGrid columns={{ base: 1, md: 2 }} gap={4}>
              <Field.Root>
                <Field.Label>Target roles</Field.Label>
                <TagInput value={draft.preferences.targetRoles} onChange={(v) => update((p) => { p.preferences.targetRoles = v; })} placeholder="e.g. Full Stack Developer" />
                <Field.HelperText>Used to build search queries. Up to four are searched per source.</Field.HelperText>
              </Field.Root>
              <Field.Root>
                <Field.Label>Preferred locations</Field.Label>
                <TagInput value={draft.preferences.preferredLocations} onChange={(v) => update((p) => { p.preferences.preferredLocations = v; })} placeholder="e.g. Mumbai, Remote" />
              </Field.Root>
              <Field.Root>
                <Field.Label>Remote preference</Field.Label>
                <NativeSelect.Root>
                  <NativeSelect.Field value={draft.preferences.remotePreference} onChange={(e) => update((p) => { p.preferences.remotePreference = e.target.value as RemotePreference; })}>
                    <option value="ANY">Any</option>
                    <option value="REMOTE">Remote only</option>
                    <option value="HYBRID">Hybrid</option>
                    <option value="ONSITE">On-site</option>
                  </NativeSelect.Field>
                  <NativeSelect.Indicator />
                </NativeSelect.Root>
              </Field.Root>
              <Field.Root>
                <Field.Label>Relocation</Field.Label>
                <NativeSelect.Root>
                  <NativeSelect.Field value={draft.preferences.relocation} onChange={(e) => update((p) => { p.preferences.relocation = e.target.value as RelocationPreference; })}>
                    <option value="NOT_SPECIFIED">Not specified</option>
                    <option value="YES">Willing to relocate</option>
                    <option value="MAYBE">Maybe</option>
                    <option value="NO">No</option>
                  </NativeSelect.Field>
                  <NativeSelect.Indicator />
                </NativeSelect.Root>
              </Field.Root>
              <Field.Root>
                <Field.Label>Employment types</Field.Label>
                <HStack wrap="wrap" gap={2}>
                  {EMPLOYMENT.map((t) => {
                    const on = draft.preferences.employmentTypes.includes(t);
                    return (
                      <Button key={t} size="xs" variant={on ? "solid" : "outline"} colorPalette={on ? "brand" : "gray"} onClick={() => update((p) => { p.preferences.employmentTypes = on ? p.preferences.employmentTypes.filter((x) => x !== t) : [...p.preferences.employmentTypes, t]; })}>
                        {t.replace("_", " ").toLowerCase()}
                      </Button>
                    );
                  })}
                </HStack>
              </Field.Root>
              <Field.Root>
                <Field.Label>Notice period</Field.Label>
                <Input value={draft.preferences.noticePeriod} placeholder="e.g. 30 days" onChange={(e) => update((p) => { p.preferences.noticePeriod = e.target.value; })} />
              </Field.Root>
              <Field.Root>
                <Field.Label>Minimum experience roles should require (years)</Field.Label>
                <Input type="number" value={draft.preferences.minimumExperienceYears ?? ""} onChange={(e) => update((p) => { p.preferences.minimumExperienceYears = e.target.value === "" ? null : Number(e.target.value); })} />
              </Field.Root>
              <Field.Root>
                <Field.Label>Salary expectation</Field.Label>
                <HStack>
                  <Input type="number" placeholder="Min" value={draft.preferences.salary.minimum ?? ""} onChange={(e) => update((p) => { p.preferences.salary.minimum = e.target.value === "" ? null : Number(e.target.value); })} />
                  <Input type="number" placeholder="Max" value={draft.preferences.salary.maximum ?? ""} onChange={(e) => update((p) => { p.preferences.salary.maximum = e.target.value === "" ? null : Number(e.target.value); })} />
                  <Input w="90px" value={draft.preferences.salary.currency} onChange={(e) => update((p) => { p.preferences.salary.currency = e.target.value; })} />
                  <NativeSelect.Root w="120px">
                    <NativeSelect.Field value={draft.preferences.salary.period} onChange={(e) => update((p) => { p.preferences.salary.period = e.target.value; })}>
                      <option value="YEAR">per year</option>
                      <option value="MONTH">per month</option>
                    </NativeSelect.Field>
                    <NativeSelect.Indicator />
                  </NativeSelect.Root>
                </HStack>
              </Field.Root>
            </SimpleGrid>
            <Field.Root mt={4}>
              <Field.Label>Notes for the agent (companies to avoid, industries, constraints)</Field.Label>
              <Textarea rows={3} value={draft.preferences.notes} onChange={(e) => update((p) => { p.preferences.notes = e.target.value; })} />
            </Field.Root>
          </Panel>
        </Tabs.Content>

        <Tabs.Content value="skills">
          <Panel>
            <SimpleGrid columns={{ base: 1, md: 2 }} gap={4}>
              {(["frontend", "backend", "database", "devops", "cloud", "testing", "other"] as const).map((group) => (
                <Field.Root key={group}>
                  <Field.Label textTransform="capitalize">{group}</Field.Label>
                  <TagInput value={draft.skills[group]} onChange={(v) => update((p) => { p.skills[group] = v; })} />
                </Field.Root>
              ))}
            </SimpleGrid>
          </Panel>
        </Tabs.Content>

        <Tabs.Content value="experience">
          <ExperienceTab />
        </Tabs.Content>
        <Tabs.Content value="projects">
          <ProjectsTab />
        </Tabs.Content>

        <Tabs.Content value="education">
          <VStack align="stretch" gap={4}>
            <Panel title="Education" action={<Button size="xs" variant="ghost" onClick={() => update((p) => { p.education.push({ id: crypto.randomUUID(), institution: "", degree: "", field: "", startDate: "", endDate: "", grade: "", description: "" }); })}><Plus size={12} /> Add</Button>}>
              {draft.education.length === 0 ? (
                <Text fontSize="sm" color="fg.muted">
                  No education added.
                </Text>
              ) : (
                <VStack align="stretch" gap={3}>
                  {draft.education.map((ed, i) => (
                    <EducationRow key={ed.id} value={ed} onChange={(v) => update((p) => { p.education[i] = v; })} onRemove={() => update((p) => { p.education.splice(i, 1); })} />
                  ))}
                </VStack>
              )}
            </Panel>
            <Panel title="Certifications" action={<Button size="xs" variant="ghost" onClick={() => update((p) => { p.certifications.push({ id: crypto.randomUUID(), name: "", issuer: "", date: "", url: "" }); })}><Plus size={12} /> Add</Button>}>
              {draft.certifications.length === 0 ? (
                <Text fontSize="sm" color="fg.muted">
                  No certifications added.
                </Text>
              ) : (
                <VStack align="stretch" gap={2}>
                  {draft.certifications.map((c, i) => (
                    <CertificationRow key={c.id} value={c} onChange={(v) => update((p) => { p.certifications[i] = v; })} onRemove={() => update((p) => { p.certifications.splice(i, 1); })} />
                  ))}
                </VStack>
              )}
            </Panel>
            <SimpleGrid columns={{ base: 1, md: 2 }} gap={4}>
              <Panel title="Achievements">
                <LinesInput value={draft.achievements} onChange={(v) => update((p) => { p.achievements = v; })} />
              </Panel>
              <Panel title="Languages">
                <TagInput value={draft.languages} onChange={(v) => update((p) => { p.languages = v; })} />
              </Panel>
            </SimpleGrid>
          </VStack>
        </Tabs.Content>

        <Tabs.Content value="resume">
          <MasterResumeTab profile={draft} />
        </Tabs.Content>
      </Tabs.Root>
    </Box>
  );
}

function EducationRow({ value, onChange, onRemove }: { value: Education; onChange: (v: Education) => void; onRemove: () => void }) {
  return (
    <SimpleGrid columns={{ base: 2, md: 6 }} gap={2} alignItems="center">
      <Input size="sm" placeholder="Institution" value={value.institution} onChange={(e) => onChange({ ...value, institution: e.target.value })} />
      <Input size="sm" placeholder="Degree" value={value.degree} onChange={(e) => onChange({ ...value, degree: e.target.value })} />
      <Input size="sm" placeholder="Field" value={value.field} onChange={(e) => onChange({ ...value, field: e.target.value })} />
      <Input size="sm" placeholder="Start (YYYY-MM)" value={value.startDate} onChange={(e) => onChange({ ...value, startDate: e.target.value })} />
      <Input size="sm" placeholder="End" value={value.endDate} onChange={(e) => onChange({ ...value, endDate: e.target.value })} />
      <HStack>
        <Input size="sm" placeholder="Grade" value={value.grade} onChange={(e) => onChange({ ...value, grade: e.target.value })} />
        <Button size="xs" variant="ghost" colorPalette="red" onClick={onRemove}>
          <Trash2 size={12} />
        </Button>
      </HStack>
    </SimpleGrid>
  );
}

function CertificationRow({ value, onChange, onRemove }: { value: Certification; onChange: (v: Certification) => void; onRemove: () => void }) {
  return (
    <SimpleGrid columns={{ base: 2, md: 4 }} gap={2} alignItems="center">
      <Input size="sm" placeholder="Name" value={value.name} onChange={(e) => onChange({ ...value, name: e.target.value })} />
      <Input size="sm" placeholder="Issuer" value={value.issuer} onChange={(e) => onChange({ ...value, issuer: e.target.value })} />
      <Input size="sm" placeholder="Date" value={value.date} onChange={(e) => onChange({ ...value, date: e.target.value })} />
      <HStack>
        <Input size="sm" placeholder="URL" value={value.url} onChange={(e) => onChange({ ...value, url: e.target.value })} />
        <Button size="xs" variant="ghost" colorPalette="red" onClick={onRemove}>
          <Trash2 size={12} />
        </Button>
      </HStack>
    </SimpleGrid>
  );
}

function emptyExperience(): Experience {
  const now = new Date().toISOString();
  return { id: "", userId: "", company: "", role: "", location: "", startDate: "", endDate: null, description: "", technologies: [], achievements: [], projects: [], createdAt: now, updatedAt: now };
}

function ExperienceTab() {
  const list = useExperiences();
  const save = useSaveExperience();
  const del = useDeleteExperience();
  const [editing, setEditing] = useState<Experience | null>(null);
  return (
    <VStack align="stretch" gap={4}>
      <ErrorBanner error={save.error ?? del.error ?? list.error} />
      {editing ? (
        <Panel title={editing.id ? "Edit experience" : "New experience"}>
          <SimpleGrid columns={{ base: 1, md: 2 }} gap={3}>
            <Field.Root><Field.Label>Company</Field.Label><Input value={editing.company} onChange={(e) => setEditing({ ...editing, company: e.target.value })} /></Field.Root>
            <Field.Root><Field.Label>Role</Field.Label><Input value={editing.role} onChange={(e) => setEditing({ ...editing, role: e.target.value })} /></Field.Root>
            <Field.Root><Field.Label>Location</Field.Label><Input value={editing.location} onChange={(e) => setEditing({ ...editing, location: e.target.value })} /></Field.Root>
            <HStack>
              <Field.Root><Field.Label>Start (YYYY-MM)</Field.Label><Input value={editing.startDate} onChange={(e) => setEditing({ ...editing, startDate: e.target.value })} /></Field.Root>
              <Field.Root><Field.Label>End (blank = current)</Field.Label><Input value={editing.endDate ?? ""} onChange={(e) => setEditing({ ...editing, endDate: e.target.value || null })} /></Field.Root>
            </HStack>
          </SimpleGrid>
          <Field.Root mt={3}><Field.Label>Description</Field.Label><Textarea rows={3} value={editing.description} onChange={(e) => setEditing({ ...editing, description: e.target.value })} /></Field.Root>
          <Field.Root mt={3}><Field.Label>Technologies</Field.Label><TagInput value={editing.technologies} onChange={(v) => setEditing({ ...editing, technologies: v })} /></Field.Root>
          <Field.Root mt={3}><Field.Label>Achievements (one per line, real numbers only)</Field.Label><LinesInput value={editing.achievements} onChange={(v) => setEditing({ ...editing, achievements: v })} /></Field.Root>
          <HStack justify="flex-end" mt={4}>
            <Button variant="ghost" onClick={() => setEditing(null)}>Cancel</Button>
            <Button colorPalette="brand" loading={save.isPending} disabled={!editing.company.trim() || !editing.role.trim()} onClick={() => save.mutate(editing, { onSuccess: () => setEditing(null) })}>Save</Button>
          </HStack>
        </Panel>
      ) : (
        <HStack justify="flex-end">
          <Button size="sm" variant="subtle" onClick={() => setEditing(emptyExperience())}><Plus size={14} /> Add experience</Button>
        </HStack>
      )}
      {list.data?.length === 0 && !editing ? <EmptyState title="No experience yet" description="Add your employers, or import your master resume and let Claude parse it." /> : null}
      {list.data?.map((e) => (
        <Panel key={e.id}>
          <HStack justify="space-between" align="flex-start">
            <Box>
              <Heading size="sm">{e.role} · {e.company}</Heading>
              <Text fontSize="sm" color="fg.muted">{e.startDate} – {e.endDate ?? "Present"}{e.location ? ` · ${e.location}` : ""}</Text>
              {e.description ? <Text fontSize="sm" mt={2} className="selectable">{e.description}</Text> : null}
              {e.technologies.length ? <Text fontSize="xs" color="fg.muted" mt={2}>{e.technologies.join(", ")}</Text> : null}
              {e.achievements.length ? <Box as="ul" pl={5} mt={2} fontSize="sm">{e.achievements.map((a, i) => <li key={i}>{a}</li>)}</Box> : null}
            </Box>
            <HStack>
              <Button size="xs" variant="ghost" onClick={() => setEditing(e)}>Edit</Button>
              <Button size="xs" variant="ghost" colorPalette="red" onClick={() => del.mutate(e.id)}><Trash2 size={12} /></Button>
            </HStack>
          </HStack>
        </Panel>
      ))}
    </VStack>
  );
}

function emptyProject(): Project {
  const now = new Date().toISOString();
  return { id: "", userId: "", experienceId: null, name: "", description: "", role: "", technologies: [], responsibilities: [], achievements: [], url: "", createdAt: now, updatedAt: now };
}

function ProjectsTab() {
  const list = useProjects();
  const experiences = useExperiences();
  const save = useSaveProject();
  const del = useDeleteProject();
  const [editing, setEditing] = useState<Project | null>(null);
  return (
    <VStack align="stretch" gap={4}>
      <ErrorBanner error={save.error ?? del.error ?? list.error} />
      {editing ? (
        <Panel title={editing.id ? "Edit project" : "New project"}>
          <SimpleGrid columns={{ base: 1, md: 2 }} gap={3}>
            <Field.Root><Field.Label>Name</Field.Label><Input value={editing.name} onChange={(e) => setEditing({ ...editing, name: e.target.value })} /></Field.Root>
            <Field.Root><Field.Label>Your role</Field.Label><Input value={editing.role} onChange={(e) => setEditing({ ...editing, role: e.target.value })} /></Field.Root>
            <Field.Root><Field.Label>URL</Field.Label><Input value={editing.url} onChange={(e) => setEditing({ ...editing, url: e.target.value })} /></Field.Root>
            <Field.Root>
              <Field.Label>Employer (optional)</Field.Label>
              <NativeSelect.Root>
                <NativeSelect.Field value={editing.experienceId ?? ""} onChange={(e) => setEditing({ ...editing, experienceId: e.target.value || null })}>
                  <option value="">Personal / other</option>
                  {experiences.data?.map((x) => <option key={x.id} value={x.id}>{x.company}</option>)}
                </NativeSelect.Field>
                <NativeSelect.Indicator />
              </NativeSelect.Root>
            </Field.Root>
          </SimpleGrid>
          <Field.Root mt={3}><Field.Label>Description</Field.Label><Textarea rows={3} value={editing.description} onChange={(e) => setEditing({ ...editing, description: e.target.value })} /></Field.Root>
          <Field.Root mt={3}><Field.Label>Technologies</Field.Label><TagInput value={editing.technologies} onChange={(v) => setEditing({ ...editing, technologies: v })} /></Field.Root>
          <SimpleGrid columns={{ base: 1, md: 2 }} gap={3} mt={3}>
            <Field.Root><Field.Label>Responsibilities</Field.Label><LinesInput value={editing.responsibilities} onChange={(v) => setEditing({ ...editing, responsibilities: v })} rows={3} /></Field.Root>
            <Field.Root><Field.Label>Achievements</Field.Label><LinesInput value={editing.achievements} onChange={(v) => setEditing({ ...editing, achievements: v })} rows={3} /></Field.Root>
          </SimpleGrid>
          <HStack justify="flex-end" mt={4}>
            <Button variant="ghost" onClick={() => setEditing(null)}>Cancel</Button>
            <Button colorPalette="brand" loading={save.isPending} disabled={!editing.name.trim()} onClick={() => save.mutate(editing, { onSuccess: () => setEditing(null) })}>Save</Button>
          </HStack>
        </Panel>
      ) : (
        <HStack justify="flex-end">
          <Button size="sm" variant="subtle" onClick={() => setEditing(emptyProject())}><Plus size={14} /> Add project</Button>
        </HStack>
      )}
      {list.data?.length === 0 && !editing ? <EmptyState title="No projects yet" description="Projects help the agent pick relevant work for each resume." /> : null}
      {list.data?.map((p) => (
        <Panel key={p.id}>
          <HStack justify="space-between" align="flex-start">
            <Box>
              <Heading size="sm">{p.name}{p.role ? <Text as="span" fontWeight="normal" color="fg.muted"> · {p.role}</Text> : null}</Heading>
              {p.description ? <Text fontSize="sm" mt={1} className="selectable">{p.description}</Text> : null}
              {p.technologies.length ? <Text fontSize="xs" color="fg.muted" mt={2}>{p.technologies.join(", ")}</Text> : null}
            </Box>
            <HStack>
              <Button size="xs" variant="ghost" onClick={() => setEditing(p)}>Edit</Button>
              <Button size="xs" variant="ghost" colorPalette="red" onClick={() => del.mutate(p.id)}><Trash2 size={12} /></Button>
            </HStack>
          </HStack>
        </Panel>
      ))}
    </VStack>
  );
}

export function MasterResumeTab({ profile, compact = false }: { profile: CandidateProfile; compact?: boolean }) {
  const importResume = useImportMasterResume();
  const parse = useParseMasterResume();
  const text = useMasterResumeText();
  const pick = async () => {
    const selected = await openDialog({ multiple: false, directory: false, filters: [{ name: "Resume", extensions: ["pdf", "docx"] }] });
    if (typeof selected === "string") importResume.mutate(selected);
  };
  const master = profile.masterResume;
  return (
    <VStack align="stretch" gap={4}>
      <ErrorBanner error={importResume.error ?? parse.error} />
      <Panel>
        <HStack justify="space-between" align="flex-start">
          <Box>
            <Heading size="sm">Master resume</Heading>
            {master ? (
              <Text fontSize="sm" color="fg.muted" mt={1}>
                {master.originalFileName} · imported {formatDateTime(master.importedAt)} · {master.textChars.toLocaleString()} characters extracted
                <br />
                Stored as {fileName(master.storedPath)} (the original file is never modified).
              </Text>
            ) : (
              <Text fontSize="sm" color="fg.muted" mt={1}>
                Import your existing PDF or DOCX resume. It is the source of truth for every tailored resume the agent generates.
              </Text>
            )}
          </Box>
          <HStack>
            <Button size="sm" variant={master ? "outline" : "solid"} colorPalette="brand" onClick={pick} loading={importResume.isPending}>
              <FileUp size={14} /> {master ? "Replace" : "Import resume"}
            </Button>
            {master ? (
              <Button size="sm" variant="subtle" onClick={() => parse.mutate()} loading={parse.isPending}>
                <Sparkles size={14} /> Fill profile with Claude
              </Button>
            ) : null}
          </HStack>
        </HStack>
        {parse.isSuccess ? (
          <InfoBanner status="success" title="Profile updated">
            Empty fields were filled from your resume. Existing values were kept. Review the other tabs and save.
          </InfoBanner>
        ) : null}
      </Panel>
      {!compact && text.data ? (
        <Panel title="Extracted text">
          <Box as="pre" fontFamily="mono" fontSize="xs" whiteSpace="pre-wrap" maxH="420px" overflowY="auto" className="selectable">
            {text.data}
          </Box>
        </Panel>
      ) : null}
    </VStack>
  );
}
