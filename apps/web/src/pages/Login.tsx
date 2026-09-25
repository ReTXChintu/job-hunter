import { Box, Button, Center, Field, Heading, Input, Text, VStack } from "@chakra-ui/react";
import { useState, type FormEvent } from "react";

import { ErrorBox } from "../components/State";
import { useSession } from "../session";

export function LoginPage() {
  const { api } = useSession();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<unknown>(null);

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      await api.login(email.trim(), password);
    } catch (err) {
      setError(err);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Center minH="100vh" px={4}>
      <Box w="full" maxW="380px" borderWidth="1px" borderColor="border.muted" borderRadius="lg" bg="bg.panel" p={{ base: 6, md: 8 }}>
        <Heading size="lg" mb={1}>
          Job Hunter
        </Heading>
        <Text fontSize="sm" color="fg.muted" mb={6}>
          Sign in with the same account you use in the desktop app to see your jobs and applications.
        </Text>
        {error ? <ErrorBox error={error} /> : null}
        <form onSubmit={submit}>
          <VStack gap={4} align="stretch">
            <Field.Root required>
              <Field.Label>Email</Field.Label>
              <Input type="email" autoComplete="username" value={email} onChange={(e) => setEmail(e.target.value)} />
            </Field.Root>
            <Field.Root required>
              <Field.Label>Password</Field.Label>
              <Input type="password" autoComplete="current-password" value={password} onChange={(e) => setPassword(e.target.value)} />
            </Field.Root>
            <Button type="submit" colorPalette="brand" loading={busy} disabled={!email.trim() || !password}>
              Sign in
            </Button>
          </VStack>
        </form>
        <Text fontSize="xs" color="fg.muted" mt={6}>
          No account yet? Create one in the desktop app under Settings → Backend account.
        </Text>
      </Box>
    </Center>
  );
}
