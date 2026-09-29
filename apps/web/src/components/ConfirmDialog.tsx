import { Button, Dialog, Portal, Text } from "@chakra-ui/react";
import type { ReactNode } from "react";

import { ErrorBox } from "./State";

/**
 * Confirms an action that the desktop will carry out. Stays open (and
 * can't be dismissed) while the request runs, and shows the desktop's error
 * in place if it fails; the caller closes it on success.
 */
export function ConfirmDialog({
  open,
  onClose,
  title,
  description,
  confirmLabel,
  destructive = false,
  loading = false,
  error,
  onConfirm,
  children,
}: {
  open: boolean;
  onClose: () => void;
  title: string;
  description?: ReactNode;
  confirmLabel: string;
  destructive?: boolean;
  loading?: boolean;
  error?: unknown;
  onConfirm: () => void;
  children?: ReactNode;
}) {
  return (
    <Dialog.Root
      open={open}
      onOpenChange={(e) => {
        if (!e.open && !loading) onClose();
      }}
      placement="center"
      role={destructive ? "alertdialog" : "dialog"}
      closeOnInteractOutside={!loading}
      closeOnEscape={!loading}
    >
      <Portal>
        <Dialog.Backdrop />
        <Dialog.Positioner px={4}>
          <Dialog.Content>
            <Dialog.Header>
              <Dialog.Title>{title}</Dialog.Title>
            </Dialog.Header>
            <Dialog.Body>
              {description ? <Text color="fg.muted">{description}</Text> : null}
              {children}
              {error ? <ErrorBox error={error} title="Your desktop couldn't do that" /> : null}
            </Dialog.Body>
            <Dialog.Footer>
              <Button variant="ghost" onClick={onClose} disabled={loading}>
                Cancel
              </Button>
              <Button colorPalette={destructive ? "red" : "brand"} onClick={onConfirm} loading={loading} loadingText="Waiting for your desktop…">
                {confirmLabel}
              </Button>
            </Dialog.Footer>
          </Dialog.Content>
        </Dialog.Positioner>
      </Portal>
    </Dialog.Root>
  );
}
