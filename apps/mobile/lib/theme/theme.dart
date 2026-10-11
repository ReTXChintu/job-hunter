import 'package:flutter/material.dart';

/// Matches the desktop app's brand indigo (`apps/desktop/src/theme.ts`) so
/// the two feel like one product.
const brandSeed = Color(0xFF4F46E5);
const _brandDeep = Color(0xFF3730A3);
const _brandBright = Color(0xFF6366F1);

/// The Home hero's background.
LinearGradient brandGradient(Brightness brightness) => LinearGradient(
      begin: Alignment.topLeft,
      end: Alignment.bottomRight,
      colors: brightness == Brightness.dark ? const [Color(0xFF312E81), Color(0xFF1E1B4B)] : const [_brandBright, _brandDeep],
    );

ThemeData buildTheme(Brightness brightness) {
  final dark = brightness == Brightness.dark;
  final scheme = ColorScheme.fromSeed(seedColor: brandSeed, brightness: brightness);
  final background = dark ? const Color(0xFF0F1117) : const Color(0xFFF4F5FA);
  final card = dark ? const Color(0xFF181B24) : Colors.white;
  final outline = dark ? const Color(0xFF2A2E3A) : const Color(0xFFE6E8F0);
  const radius = 16.0;

  return ThemeData(
    useMaterial3: true,
    colorScheme: scheme.copyWith(surface: card, surfaceContainerLow: dark ? const Color(0xFF1C2029) : const Color(0xFFF7F8FC), outlineVariant: outline),
    scaffoldBackgroundColor: background,
    splashFactory: InkSparkle.splashFactory,
    appBarTheme: AppBarTheme(
      backgroundColor: background,
      foregroundColor: scheme.onSurface,
      elevation: 0,
      scrolledUnderElevation: 0,
      centerTitle: false,
      titleTextStyle: TextStyle(fontSize: 22, fontWeight: FontWeight.w700, color: scheme.onSurface, letterSpacing: -0.3),
    ),
    cardTheme: CardThemeData(
      elevation: 0,
      color: card,
      margin: EdgeInsets.zero,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(radius), side: BorderSide(color: outline)),
    ),
    dividerTheme: DividerThemeData(color: outline, thickness: 1, space: 1),
    inputDecorationTheme: InputDecorationTheme(
      filled: true,
      fillColor: card,
      isDense: true,
      contentPadding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
      border: OutlineInputBorder(borderRadius: BorderRadius.circular(12), borderSide: BorderSide(color: outline)),
      enabledBorder: OutlineInputBorder(borderRadius: BorderRadius.circular(12), borderSide: BorderSide(color: outline)),
      focusedBorder: OutlineInputBorder(borderRadius: BorderRadius.circular(12), borderSide: BorderSide(color: scheme.primary, width: 1.5)),
    ),
    chipTheme: ChipThemeData(
      side: BorderSide(color: outline),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(20)),
      labelStyle: TextStyle(fontSize: 13, fontWeight: FontWeight.w600, color: scheme.onSurface),
      selectedColor: scheme.primaryContainer,
      backgroundColor: card,
      showCheckmark: false,
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
    ),
    filledButtonTheme: FilledButtonThemeData(
      style: FilledButton.styleFrom(
        padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 14),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
        textStyle: const TextStyle(fontWeight: FontWeight.w600, fontSize: 15),
      ),
    ),
    outlinedButtonTheme: OutlinedButtonThemeData(
      style: OutlinedButton.styleFrom(
        padding: const EdgeInsets.symmetric(horizontal: 18, vertical: 14),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
        side: BorderSide(color: outline),
        textStyle: const TextStyle(fontWeight: FontWeight.w600, fontSize: 15),
      ),
    ),
    textButtonTheme: TextButtonThemeData(
      style: TextButton.styleFrom(shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(10)), textStyle: const TextStyle(fontWeight: FontWeight.w600)),
    ),
    navigationBarTheme: NavigationBarThemeData(
      backgroundColor: card,
      indicatorColor: scheme.primaryContainer,
      surfaceTintColor: Colors.transparent,
      height: 68,
      labelTextStyle: WidgetStateProperty.resolveWith(
        (states) => TextStyle(fontSize: 11.5, fontWeight: states.contains(WidgetState.selected) ? FontWeight.w700 : FontWeight.w500),
      ),
    ),
    snackBarTheme: SnackBarThemeData(behavior: SnackBarBehavior.floating, shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12))),
    dialogTheme: DialogThemeData(shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(20))),
    bottomSheetTheme: const BottomSheetThemeData(shape: RoundedRectangleBorder(borderRadius: BorderRadius.vertical(top: Radius.circular(24)))),
    listTileTheme: const ListTileThemeData(contentPadding: EdgeInsets.symmetric(horizontal: 16, vertical: 2)),
    progressIndicatorTheme: ProgressIndicatorThemeData(linearTrackColor: scheme.primary.withValues(alpha: 0.15), borderRadius: BorderRadius.circular(4)),
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

/// The colour of a match score, as on the desktop's MatchScore.
Color matchColor(int? score, bool? relevant) {
  if (score == null || relevant == false) return Colors.grey;
  if (score >= 85) return Colors.green;
  if (score >= 60) return Colors.blue;
  return Colors.orange;
}
