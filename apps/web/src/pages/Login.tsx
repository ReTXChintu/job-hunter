import { Box, Button, Center, Field, Heading, Input, SegmentGroup, Text, VStack } from "@chakra-ui/react";
import { useState, type FormEvent } from "react";

import { AppDownloads, useDownloads } from "../components/AppDownloads";
import { ErrorBox } from "../components/State";
import { useSession } from "../session";

type Mode = "login" | "register";

export function LoginPage() {
  const { api } = useSession();
  const downloads = useDownloads();
  const [mode, setMode] = useState<Mode>("login");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<unknown>(null);

  const registering = mode === "register";
  const mismatch = registering && confirm.length > 0 && confirm !== password;
  const canSubmit = !!email.trim() && (registering ? password.length >= 8 && confirm === password : !!password);

  const switchMode = (next: Mode) => {
    setMode(next);
    setError(null);
    setConfirm("");
  };

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    if (!canSubmit) return;
    setBusy(true);
    setError(null);
    try {
      if (registering) await api.register(email.trim(), password);
      else await api.login(email.trim(), password);
    } catch (err) {
      setError(err);
    } finally {
      setBusy(false);
    }
  };

  const hasDownloads = !!(downloads.data?.android || downloads.data?.windows);

  return (
    <Center minH="100vh" px={4} py={8}>
      <VStack w="full" maxW="400px" gap={4} align="stretch">
        <Box borderWidth="1px" borderColor="border.muted" borderRadius="lg" bg="bg.panel" p={{ base: 6, md: 8 }}>
          <Heading size="lg" mb={1}>
            Job Hunter
          </Heading>
          <Text fontSize="sm" color="fg.muted" mb={5}>
            {registering
              ? "Create your account. Then sign in to the same account in the desktop app (Settings → Backend account) so your jobs sync here."
              : "Sign in with the same account you use in the desktop app to see your jobs and applications."}
          </Text>
          <SegmentGroup.Root value={mode} onValueChange={(e) => switchMode((e.value as Mode) ?? "login")} size="sm" mb={5} w="full">
            <SegmentGroup.Indicator />
            <SegmentGroup.Items
              flex="1"
              items={[
                { value: "login", label: "Sign in" },
                { value: "register", label: "Create account" },
              ]}
            />
          </SegmentGroup.Root>
          {error ? <ErrorBox error={error} title={registering ? "Couldn't create the account" : "Couldn't sign in"} /> : null}
          <form onSubmit={submit}>
            <VStack gap={4} align="stretch">
              <Field.Root required>
                <Field.Label>Email</Field.Label>
                <Input type="email" autoComplete={registering ? "email" : "username"} value={email} onChange={(e) => setEmail(e.target.value)} />
              </Field.Root>
              <Field.Root required>
                <Field.Label>Password</Field.Label>
                <Input
                  type="password"
                  autoComplete={registering ? "new-password" : "current-password"}
                  placeholder={registering ? "At least 8 characters" : undefined}
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                />
              </Field.Root>
              {registering ? (
                <Field.Root required invalid={mismatch}>
                  <Field.Label>Confirm password</Field.Label>
                  <Input type="password" autoComplete="new-password" value={confirm} onChange={(e) => setConfirm(e.target.value)} />
                  {mismatch ? <Field.ErrorText>Passwords don&apos;t match.</Field.ErrorText> : null}
                </Field.Root>
              ) : null}
              <Button type="submit" colorPalette="brand" loading={busy} disabled={!canSubmit}>
                {registering ? "Create account" : "Sign in"}
              </Button>
            </VStack>
          </form>
        </Box>

        {hasDownloads ? (
          <Box borderWidth="1px" borderColor="border.muted" borderRadius="lg" bg="bg.panel" p={{ base: 5, md: 6 }}>
            <Text fontSize="sm" fontWeight="semibold" mb={3}>
              Get the apps
            </Text>
            <AppDownloads compact />
          </Box>
        ) : null}
      </VStack>
    </Center>
  );
}
