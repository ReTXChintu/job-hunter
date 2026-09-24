import 'package:flutter/material.dart';

/// Matches the desktop app's brand indigo (`apps/desktop/src/theme.ts`) so
/// the two feel like one product.
const _brandSeed = Color(0xFF4F46E5);

ThemeData buildTheme(Brightness brightness) {
  final colorScheme = ColorScheme.fromSeed(seedColor: _brandSeed, brightness: brightness);
  return ThemeData(
    useMaterial3: true,
    colorScheme: colorScheme,
    scaffoldBackgroundColor: brightness == Brightness.dark ? const Color(0xFF14161D) : Colors.white,
    appBarTheme: AppBarTheme(backgroundColor: brightness == Brightness.dark ? const Color(0xFF0F1117) : const Color(0xFFF6F7FB), foregroundColor: colorScheme.onSurface, elevation: 0, centerTitle: false),
    cardTheme: const CardThemeData(elevation: 0, shape: RoundedRectangleBorder(borderRadius: BorderRadius.all(Radius.circular(10)))),
    inputDecorationTheme: const InputDecorationTheme(border: OutlineInputBorder(borderRadius: BorderRadius.all(Radius.circular(8)))),
    filledButtonTheme: FilledButtonThemeData(style: FilledButton.styleFrom(padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 14), shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)))),
  );
}

/// Status colours shared with `packages/shared`'s `statusTone` so the same
/// status always reads the same way on both apps.
Color statusColor(BuildContext context, String status) {
  final scheme = Theme.of(context).colorScheme;
  switch (status) {
    case 'APPLIED':
    case 'INTERVIEW':
    case 'OFFER':
      return Colors.green;
    case 'READY_FOR_REVIEW':
    case 'APPROVED':
    case 'APPLYING':
      return scheme.primary;
    case 'MANUAL_ACTION_REQUIRED':
    case 'WAITING_FOR_USER':
      return Colors.orange;
    case 'REJECTED':
    case 'NOT_RELEVANT':
    case 'WITHDRAWN':
      return Colors.redAccent;
    case 'SHORTLISTED':
    case 'ANALYZED':
      return Colors.blue;
    default:
      return scheme.onSurfaceVariant;
  }
}

const statusLabels = <String, String>{
  'DISCOVERED': 'Discovered',
  'ANALYZED': 'Analyzed',
  'NOT_RELEVANT': 'Not relevant',
  'SHORTLISTED': 'Shortlisted',
  'READY_FOR_REVIEW': 'Awaiting approval',
  'APPROVED': 'Approved',
  'APPLYING': 'Applying',
  'APPLIED': 'Applied',
  'MANUAL_ACTION_REQUIRED': 'Manual action',
  'WAITING_FOR_USER': 'Needs your input',
  'REJECTED': 'Rejected',
  'INTERVIEW': 'Interview',
  'OFFER': 'Offer',
  'WITHDRAWN': 'Withdrawn',
};

String statusLabel(String status) => statusLabels[status] ?? status;
