import { Box, Button, Field, HStack, Image, Input, SegmentGroup, Spinner, Table, Text, VStack } from "@chakra-ui/react";
import { formatDateTime, timeAgo } from "@job-hunter/shared";
import type { RemoteDevice } from "@job-hunter/types";
import QRCode from "qrcode";
import { LogIn, LogOut, Smartphone, UserPlus } from "lucide-react";
import { useEffect, useState } from "react";
import { ConfirmDialog, ErrorBanner, InfoBanner, Panel } from "./common";
import { useRemoteCreatePairingCode, useRemoteDevices, useRemoteLogin, useRemoteLogout, useRemoteRegister, useRemoteRevokeDevice, useRemoteStatus } from "../lib/queries";

/**
 * Settings → Mobile app. Configures the connection to a self-hosted
 * job-hunter-relay: sign up or sign in once, then pair phones with a
 * one-time code (no password ever typed on the phone). The relay never
 * sees job data or runs any AI -- see docs/mobile-protocol.md.
 */
export function MobileAppSettings() {
  const status = useRemoteStatus();
  if (!status.data) return status.error ? <ErrorBanner error={status.error} /> : <Spinner size="sm" />;
  return status.data.configured ? <ConnectedView status={status.data} /> : <SignInForm defaultDeviceName={status.data.deviceName} />;
}

function SignInForm({ defaultDeviceName }: { defaultDeviceName: string }) {
  const [mode, setMode] = useState<"register" | "login">("register");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [deviceName, setDeviceName] = useState(defaultDeviceName);
  const register = useRemoteRegister();
  const login = useRemoteLogin();
  const active = mode === "register" ? register : login;

  const submit = () => active.mutate({ email: email.trim(), password, deviceName: deviceName.trim() || undefined });

  return (
    <Panel title="Mobile app">
      <Text fontSize="sm" color="fg.muted" mb={4}>
        Review and approve applications from your phone, wherever you are. This connects to your own Job Hunter server, which only routes messages between this computer and your phone. Nothing here runs any AI. Use the same account as Settings → Backend account.
      </Text>
      <ErrorBanner error={register.error ?? login.error} />
      <VStack align="stretch" gap={3} maxW="480px">
        <SegmentGroup.Root value={mode} onValueChange={(e) => setMode((e.value as "register" | "login") ?? "register")} size="sm">
          <SegmentGroup.Indicator />
          <SegmentGroup.Items items={[{ value: "register", label: "Create account" }, { value: "login", label: "Sign in" }]} />
        </SegmentGroup.Root>
        <Field.Root>
          <Field.Label>Email</Field.Label>
          <Input type="email" value={email} onChange={(e) => setEmail(e.target.value)} />
        </Field.Root>
        <Field.Root>
          <Field.Label>Password</Field.Label>
          <Input type="password" value={password} onChange={(e) => setPassword(e.target.value)} placeholder={mode === "register" ? "At least 8 characters" : undefined} />
        </Field.Root>
        <Field.Root>
          <Field.Label>This computer&apos;s name</Field.Label>
          <Input value={deviceName} onChange={(e) => setDeviceName(e.target.value)} placeholder="e.g. Work Laptop" />
          <Field.HelperText>Shown to your phone and in the paired-devices list below.</Field.HelperText>
        </Field.Root>
        <HStack>
          <Button colorPalette="brand" onClick={submit} loading={active.isPending} disabled={!email.trim() || password.length < 8}>
            {mode === "register" ? <UserPlus size={14} /> : <LogIn size={14} />}
            {mode === "register" ? "Create account & connect" : "Sign in & connect"}
          </Button>
        </HStack>
      </VStack>
    </Panel>
  );
}

function ConnectedView({ status }: { status: NonNullable<ReturnType<typeof useRemoteStatus>["data"]> }) {
  const logout = useRemoteLogout();
  const [confirmSignOut, setConfirmSignOut] = useState(false);

  return (
    <VStack align="stretch" gap={4}>
      <Panel
        title="Mobile app"
        action={
          <Button size="xs" variant="outline" colorPalette="red" onClick={() => setConfirmSignOut(true)}>
            <LogOut size={12} /> Sign out
          </Button>
        }
      >
        <ErrorBanner error={logout.error} />
        <HStack gap={6} fontSize="sm" wrap="wrap">
          <HStack>
            <Box w={2} h={2} borderRadius="full" bg={status.connected ? "green.solid" : "orange.solid"} />
            <Text fontWeight="semibold">{status.connected ? "Connected" : "Not connected"}</Text>
          </HStack>
          <Text color="fg.muted">
            Account: <Text as="span" color="fg">{status.accountEmail}</Text>
          </Text>
          <Text color="fg.muted">
            This computer: <Text as="span" color="fg">{status.deviceName}</Text>
          </Text>
          <Text color="fg.muted" fontFamily="mono" fontSize="xs">
            {status.relayUrl}
          </Text>
        </HStack>
        {status.lastError ? (
          <InfoBanner status="warning" title="Last connection error">
            {status.lastError}
          </InfoBanner>
        ) : null}
      </Panel>

      <PairingPanel />
      <DevicesPanel />

      <ConfirmDialog
        open={confirmSignOut}
        onOpenChange={setConfirmSignOut}
        title="Sign out of the mobile app?"
        description="This forgets your account on this computer. Your phones stay paired until you revoke them below or sign in again and remove them."
        confirmLabel="Sign out"
        destructive
        loading={logout.isPending}
        onConfirm={() => logout.mutate(undefined, { onSuccess: () => setConfirmSignOut(false) })}
      />
    </VStack>
  );
}

function PairingPanel() {
  const create = useRemoteCreatePairingCode();
  const [qrDataUrl, setQrDataUrl] = useState<string | null>(null);
  const [secondsLeft, setSecondsLeft] = useState(0);

  useEffect(() => {
    if (!create.data) {
      setQrDataUrl(null);
      return;
    }
    let cancelled = false;
    QRCode.toDataURL(JSON.stringify({ code: create.data.code }), { width: 200, margin: 1 })
      .then((url) => { if (!cancelled) setQrDataUrl(url); })
      .catch(() => { if (!cancelled) setQrDataUrl(null); });
    return () => { cancelled = true; };
  }, [create.data]);

  useEffect(() => {
    if (!create.data) return;
    const tick = () => setSecondsLeft(Math.max(0, Math.round((new Date(create.data!.expiresAt).getTime() - Date.now()) / 1000)));
    tick();
    const id = setInterval(tick, 1000);
    return () => clearInterval(id);
  }, [create.data]);

  const expired = !!create.data && secondsLeft <= 0;

  return (
    <Panel title="Add a phone">
      <ErrorBanner error={create.error} />
      <Text fontSize="sm" color="fg.muted" mb={4}>
        Open Job Hunter on your phone, choose &quot;Pair with a code&quot;, and either scan the QR code or type the 6-character code. No password needed on the phone.
      </Text>
      {!create.data || expired ? (
        <Button variant="subtle" onClick={() => create.mutate()} loading={create.isPending}>
          <Smartphone size={14} /> {expired ? "Generate a new code" : "Generate pairing code"}
        </Button>
      ) : (
        <HStack align="flex-start" gap={6} wrap="wrap">
          {qrDataUrl ? <Image src={qrDataUrl} alt="Pairing QR code" borderRadius="md" borderWidth="1px" borderColor="border.muted" w="200px" h="200px" /> : null}
          <VStack align="flex-start" gap={2}>
            <Text fontSize="xs" color="fg.muted" textTransform="uppercase" letterSpacing="wide" fontWeight="semibold">
              Pairing code
            </Text>
            <Text fontSize="3xl" fontWeight="bold" fontFamily="mono" letterSpacing="0.2em">
              {create.data.code}
            </Text>
            <Text fontSize="xs" color="fg.muted">
              Expires in {secondsLeft}s · one-time use
            </Text>
            <Button size="xs" variant="ghost" onClick={() => create.mutate()} loading={create.isPending}>
              Generate a different code
            </Button>
          </VStack>
        </HStack>
      )}
    </Panel>
  );
}

function DevicesPanel() {
  const devices = useRemoteDevices(true);
  const revoke = useRemoteRevokeDevice();
  const [pendingRevoke, setPendingRevoke] = useState<RemoteDevice | null>(null);

  return (
    <Panel title="Paired devices" p={0}>
      <Box p={5} pb={devices.data?.length ? 0 : 5}>
        <ErrorBanner error={devices.error ?? revoke.error} />
        {!devices.data ? <Spinner size="sm" /> : devices.data.length === 0 ? <Text fontSize="sm" color="fg.muted">No devices yet.</Text> : null}
      </Box>
      {devices.data && devices.data.length > 0 ? (
        <Table.Root size="sm">
          <Table.Header>
            <Table.Row>
              <Table.ColumnHeader>Device</Table.ColumnHeader>
              <Table.ColumnHeader>Kind</Table.ColumnHeader>
              <Table.ColumnHeader>Platform</Table.ColumnHeader>
              <Table.ColumnHeader>Status</Table.ColumnHeader>
              <Table.ColumnHeader>Last seen</Table.ColumnHeader>
              <Table.ColumnHeader></Table.ColumnHeader>
            </Table.Row>
          </Table.Header>
          <Table.Body>
            {devices.data.map((d) => (
              <Table.Row key={d.id}>
                <Table.Cell>
                  <HStack gap={2}>
                    {d.kind === "mobile" ? <Smartphone size={14} /> : null}
                    <Text fontWeight="medium">{d.name}</Text>
                  </HStack>
                </Table.Cell>
                <Table.Cell textTransform="capitalize">{d.kind}</Table.Cell>
                <Table.Cell>{d.platform || "—"}</Table.Cell>
                <Table.Cell>
                  <HStack gap={1.5}>
                    <Box w={1.5} h={1.5} borderRadius="full" bg={d.online ? "green.solid" : "gray.solid"} />
                    <Text fontSize="xs" color={d.online ? "green.fg" : "fg.muted"}>
                      {d.online ? "Online" : "Offline"}
                    </Text>
                  </HStack>
                </Table.Cell>
                <Table.Cell>
                  <Text fontSize="xs" color="fg.muted" title={d.lastSeenAt ? formatDateTime(d.lastSeenAt) : undefined}>
                    {d.lastSeenAt ? timeAgo(d.lastSeenAt) : "Never"}
                  </Text>
                </Table.Cell>
                <Table.Cell>
                  <Button size="2xs" variant="ghost" colorPalette="red" onClick={() => setPendingRevoke(d)}>
                    Revoke
                  </Button>
                </Table.Cell>
              </Table.Row>
            ))}
          </Table.Body>
        </Table.Root>
      ) : null}
      <ConfirmDialog
        open={!!pendingRevoke}
        onOpenChange={(open) => !open && setPendingRevoke(null)}
        title={`Revoke "${pendingRevoke?.name}"?`}
        description={pendingRevoke?.kind === "desktop" ? "This is this computer's own device entry — revoking it will sign you out of the mobile app here." : "This phone will be disconnected immediately and will need a new pairing code to reconnect."}
        confirmLabel="Revoke"
        destructive
        loading={revoke.isPending}
        onConfirm={() => pendingRevoke && revoke.mutate(pendingRevoke.id, { onSuccess: () => setPendingRevoke(null) })}
      />
    </Panel>
  );
}
