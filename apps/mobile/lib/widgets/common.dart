import 'package:flutter/material.dart';

import '../logic/labels.dart';
import '../models/envelope.dart';
import '../state/data_source.dart';

/// The brand mark (the app icon's artwork).
class BrandLogo extends StatelessWidget {
  final double size;
  const BrandLogo({super.key, this.size = 40});

  @override
  Widget build(BuildContext context) => ClipRRect(
        borderRadius: BorderRadius.circular(size * 0.22),
        child: Image.asset('assets/logo.png', width: size, height: size, fit: BoxFit.cover, semanticLabel: 'Job Hunter'),
      );
}

/// A titled block on a detail screen.
class Section extends StatelessWidget {
  final String title;
  final Widget child;
  final Widget? trailing;
  const Section({super.key, required this.title, required this.child, this.trailing});

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(top: 20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(children: [
            Expanded(child: Text(title, style: Theme.of(context).textTheme.titleSmall?.copyWith(fontWeight: FontWeight.w700))),
            ?trailing,
          ]),
          const SizedBox(height: 8),
          child,
        ],
      ),
    );
  }
}

class BulletList extends StatelessWidget {
  final List<String> items;
  const BulletList(this.items, {super.key});

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: items
          .map((i) => Padding(
                padding: const EdgeInsets.only(bottom: 4),
                child: Row(crossAxisAlignment: CrossAxisAlignment.start, children: [const Text('•  '), Expanded(child: Text(i))]),
              ))
          .toList(),
    );
  }
}

class ChipRow extends StatelessWidget {
  final List<String> items;
  final Color color;
  const ChipRow(this.items, {super.key, required this.color});

  @override
  Widget build(BuildContext context) {
    return Wrap(
      spacing: 6,
      runSpacing: 6,
      children: items.map((i) => Chip(label: Text(i, style: const TextStyle(fontSize: 12)), backgroundColor: color.withValues(alpha: 0.12), side: BorderSide.none, visualDensity: VisualDensity.compact)).toList(),
    );
  }
}

/// A small coloured label ("email", "In Spam", "3 replies").
class Tag extends StatelessWidget {
  final String text;
  final Color color;
  final IconData? icon;
  const Tag(this.text, {super.key, required this.color, this.icon});

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 7, vertical: 2),
      decoration: BoxDecoration(color: color.withValues(alpha: 0.13), borderRadius: BorderRadius.circular(6)),
      child: Row(mainAxisSize: MainAxisSize.min, children: [
        if (icon != null) ...[Icon(icon, size: 12, color: color), const SizedBox(width: 3)],
        Text(text, style: TextStyle(color: color, fontSize: 11.5, fontWeight: FontWeight.w600)),
      ]),
    );
  }
}

/// Long text that starts collapsed with a "Show more" toggle.
class ExpandableText extends StatefulWidget {
  final String text;
  final int collapsedLines;
  const ExpandableText(this.text, {super.key, this.collapsedLines = 8});

  @override
  State<ExpandableText> createState() => _ExpandableTextState();
}

class _ExpandableTextState extends State<ExpandableText> {
  bool _expanded = false;

  @override
  Widget build(BuildContext context) {
    final long = widget.text.length > 400 || '\n'.allMatches(widget.text).length > widget.collapsedLines;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SelectableText.rich(TextSpan(text: _expanded || !long ? widget.text : _clip(widget.text)), style: Theme.of(context).textTheme.bodyMedium),
        if (long)
          TextButton(
            style: TextButton.styleFrom(padding: EdgeInsets.zero, visualDensity: VisualDensity.compact),
            onPressed: () => setState(() => _expanded = !_expanded),
            child: Text(_expanded ? 'Show less' : 'Show more'),
          ),
      ],
    );
  }

  String _clip(String s) {
    final lines = s.split('\n');
    var clipped = lines.take(widget.collapsedLines).join('\n');
    if (clipped.length > 400) clipped = clipped.substring(0, 400);
    return '${clipped.trimRight()}…';
  }
}

/// A thin strip saying where the data on screen came from when it isn't
/// live from the desktop.
class DataSourceBanner extends StatelessWidget {
  final DataSource source;
  const DataSourceBanner(this.source, {super.key});

  @override
  Widget build(BuildContext context) {
    final text = switch (source) {
      DataSource.server => 'Showing data from the server — desktop offline',
      DataSource.cache => 'Showing saved data from your last visit',
      _ => null,
    };
    if (text == null) return const SizedBox.shrink();
    final scheme = Theme.of(context).colorScheme;
    return Container(
      width: double.infinity,
      color: scheme.surfaceContainerHighest,
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 6),
      child: Row(children: [
        Icon(Icons.cloud_outlined, size: 14, color: scheme.onSurfaceVariant),
        const SizedBox(width: 8),
        Expanded(child: Text(text, style: Theme.of(context).textTheme.bodySmall)),
      ]),
    );
  }
}

/// A card-shaped callout (warnings, "needs you", explanations).
class Callout extends StatelessWidget {
  final IconData icon;
  final Color color;
  final String title;
  final String? message;
  final Widget? child;
  const Callout({super.key, required this.icon, required this.color, required this.title, this.message, this.child});

  @override
  Widget build(BuildContext context) {
    return Container(
      margin: const EdgeInsets.only(top: 16),
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(color: color.withValues(alpha: 0.09), borderRadius: BorderRadius.circular(12), border: Border.all(color: color.withValues(alpha: 0.35))),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(crossAxisAlignment: CrossAxisAlignment.start, children: [
            Icon(icon, color: color, size: 20),
            const SizedBox(width: 10),
            Expanded(child: Text(title, style: Theme.of(context).textTheme.titleSmall?.copyWith(fontWeight: FontWeight.w700))),
          ]),
          if (message != null && message!.isNotEmpty) ...[const SizedBox(height: 6), Text(message!)],
          if (child != null) ...[const SizedBox(height: 10), child!],
        ],
      ),
    );
  }
}

/// Shows the outcome of a desktop request in a SnackBar. Returns whether it
/// succeeded.
bool showOutcome(BuildContext context, RelayResponse resp, {String? success}) {
  final messenger = ScaffoldMessenger.maybeOf(context);
  if (resp.ok) {
    if (success != null) messenger?.showSnackBar(SnackBar(content: Text(success)));
    return true;
  }
  messenger?.showSnackBar(SnackBar(content: Text(friendlyError(resp)), duration: const Duration(seconds: 6)));
  return false;
}

/// A yes/no confirmation. Returns true only on confirm.
Future<bool> confirmDialog(BuildContext context, {required String title, required String message, String confirm = 'Confirm'}) async {
  final result = await showDialog<bool>(
    context: context,
    builder: (ctx) => AlertDialog(
      title: Text(title),
      content: Text(message),
      actions: [
        TextButton(onPressed: () => Navigator.of(ctx).pop(false), child: const Text('Cancel')),
        FilledButton(onPressed: () => Navigator.of(ctx).pop(true), child: Text(confirm)),
      ],
    ),
  );
  return result ?? false;
}

/// A confirmation with a text field (a reason or a note). Returns null when
/// cancelled, else the (possibly empty) text.
Future<String?> textConfirmDialog(BuildContext context, {required String title, required String message, required String label, String confirm = 'Confirm', bool required = false}) {
  return showDialog<String>(
    context: context,
    builder: (ctx) => _TextConfirmDialog(title: title, message: message, label: label, confirm: confirm, required: required),
  );
}

class _TextConfirmDialog extends StatefulWidget {
  final String title, message, label, confirm;
  final bool required;
  const _TextConfirmDialog({required this.title, required this.message, required this.label, required this.confirm, required this.required});

  @override
  State<_TextConfirmDialog> createState() => _TextConfirmDialogState();
}

class _TextConfirmDialogState extends State<_TextConfirmDialog> {
  final _controller = TextEditingController();

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final empty = _controller.text.trim().isEmpty;
    return AlertDialog(
      title: Text(widget.title),
      content: SingleChildScrollView(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(widget.message),
            const SizedBox(height: 12),
            TextField(controller: _controller, decoration: InputDecoration(labelText: widget.label), maxLines: 3, minLines: 1, onChanged: (_) => setState(() {})),
          ],
        ),
      ),
      actions: [
        TextButton(onPressed: () => Navigator.of(context).pop(), child: const Text('Cancel')),
        FilledButton(onPressed: widget.required && empty ? null : () => Navigator.of(context).pop(_controller.text.trim()), child: Text(widget.confirm)),
      ],
    );
  }
}
