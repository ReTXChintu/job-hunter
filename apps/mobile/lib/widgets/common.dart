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

/// A rounded, outlined surface: the building block of every screen.
class AppCard extends StatelessWidget {
  final Widget child;
  final EdgeInsetsGeometry padding;
  final Color? color;
  final VoidCallback? onTap;
  const AppCard({super.key, required this.child, this.padding = const EdgeInsets.all(16), this.color, this.onTap});

  @override
  Widget build(BuildContext context) {
    final body = Padding(padding: padding, child: child);
    return Card(
      color: color,
      clipBehavior: Clip.antiAlias,
      child: onTap == null ? body : InkWell(onTap: onTap, child: body),
    );
  }
}

/// A section title with an optional trailing action, above a card.
class SectionHeader extends StatelessWidget {
  final String title;
  final String? subtitle;
  final Widget? action;
  const SectionHeader(this.title, {super.key, this.subtitle, this.action});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.fromLTRB(4, 20, 0, 10),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.end,
        children: [
          Expanded(
            child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
              Text(title, style: theme.textTheme.titleMedium?.copyWith(fontWeight: FontWeight.w700, letterSpacing: -0.2)),
              if (subtitle != null) Text(subtitle!, style: theme.textTheme.bodySmall?.copyWith(color: theme.colorScheme.onSurfaceVariant)),
            ]),
          ),
          ?action,
        ],
      ),
    );
  }
}

/// An icon in a tinted circle.
class IconBubble extends StatelessWidget {
  final IconData icon;
  final Color color;
  final double size;
  const IconBubble(this.icon, {super.key, required this.color, this.size = 40});

  @override
  Widget build(BuildContext context) => Container(
        width: size,
        height: size,
        decoration: BoxDecoration(color: color.withValues(alpha: 0.14), shape: BoxShape.circle),
        child: Icon(icon, size: size * 0.5, color: color),
      );
}

/// A number with a label, for the Home stats.
class StatTile extends StatelessWidget {
  final String label;
  final int count;
  final IconData icon;
  final Color color;
  final VoidCallback? onTap;
  const StatTile({super.key, required this.label, required this.count, required this.icon, required this.color, this.onTap});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final active = count > 0;
    return AppCard(
      onTap: onTap,
      padding: const EdgeInsets.fromLTRB(12, 12, 12, 10),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          IconBubble(icon, color: active ? color : theme.colorScheme.onSurfaceVariant, size: 30),
          const SizedBox(height: 10),
          Text('$count', style: theme.textTheme.headlineSmall?.copyWith(fontWeight: FontWeight.w800, color: active ? color : theme.colorScheme.onSurfaceVariant, height: 1)),
          const SizedBox(height: 4),
          Text(label, maxLines: 2, overflow: TextOverflow.ellipsis, style: theme.textTheme.bodySmall?.copyWith(color: theme.colorScheme.onSurfaceVariant, fontWeight: FontWeight.w600, height: 1.15)),
        ],
      ),
    );
  }
}

/// One thing the desktop can do, as a tappable tile.
class ActionTile extends StatelessWidget {
  final String title;
  final String subtitle;
  final IconData icon;
  final Color color;
  final bool busy;
  final bool primary;
  final VoidCallback? onTap;
  const ActionTile({super.key, required this.title, required this.subtitle, required this.icon, required this.color, this.busy = false, this.primary = false, this.onTap});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final enabled = onTap != null;
    final fg = primary ? Colors.white : theme.colorScheme.onSurface;
    final muted = primary ? Colors.white.withValues(alpha: 0.8) : theme.colorScheme.onSurfaceVariant;
    return Opacity(
      opacity: enabled || busy ? 1 : 0.55,
      child: AppCard(
        onTap: onTap,
        color: primary ? theme.colorScheme.primary : null,
        padding: const EdgeInsets.all(14),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(children: [
              busy
                  ? SizedBox(width: 34, height: 34, child: Padding(padding: const EdgeInsets.all(7), child: CircularProgressIndicator(strokeWidth: 2.5, color: primary ? Colors.white : color)))
                  : IconBubble(icon, color: primary ? Colors.white : color, size: 34),
              const Spacer(),
              Icon(Icons.arrow_outward, size: 16, color: muted),
            ]),
            const SizedBox(height: 12),
            Text(title, maxLines: 1, overflow: TextOverflow.ellipsis, style: theme.textTheme.titleSmall?.copyWith(fontWeight: FontWeight.w700, color: fg)),
            const SizedBox(height: 2),
            Text(subtitle, maxLines: 2, overflow: TextOverflow.ellipsis, style: theme.textTheme.bodySmall?.copyWith(color: muted, height: 1.2)),
          ],
        ),
      ),
    );
  }
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
      padding: const EdgeInsets.only(top: 14),
      child: AppCard(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(children: [
              Expanded(child: Text(title, style: Theme.of(context).textTheme.titleSmall?.copyWith(fontWeight: FontWeight.w700))),
              ?trailing,
            ]),
            const SizedBox(height: 10),
            child,
          ],
        ),
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
      children: items.map((i) => Tag(i, color: color)).toList(),
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
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
      decoration: BoxDecoration(color: color.withValues(alpha: 0.13), borderRadius: BorderRadius.circular(8)),
      child: Row(mainAxisSize: MainAxisSize.min, children: [
        if (icon != null) ...[Icon(icon, size: 12, color: color), const SizedBox(width: 3)],
        Text(text, style: TextStyle(color: color, fontSize: 11.5, fontWeight: FontWeight.w700)),
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
    return InlineNotice(icon: Icons.cloud_outlined, text: text, color: Theme.of(context).colorScheme.onSurfaceVariant);
  }
}

/// A one-line notice strip between the app bar and the content.
class InlineNotice extends StatelessWidget {
  final IconData icon;
  final String text;
  final Color color;
  final Widget? trailing;
  const InlineNotice({super.key, required this.icon, required this.text, required this.color, this.trailing});

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 6, 16, 2),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
        decoration: BoxDecoration(color: color.withValues(alpha: 0.1), borderRadius: BorderRadius.circular(10)),
        child: Row(children: [
          Icon(icon, size: 16, color: color),
          const SizedBox(width: 8),
          Expanded(child: Text(text, style: TextStyle(color: color, fontSize: 12.5, fontWeight: FontWeight.w600))),
          ?trailing,
        ]),
      ),
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
      margin: const EdgeInsets.only(top: 14),
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(color: color.withValues(alpha: 0.08), borderRadius: BorderRadius.circular(16), border: Border.all(color: color.withValues(alpha: 0.35))),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(crossAxisAlignment: CrossAxisAlignment.center, children: [
            IconBubble(icon, color: color, size: 32),
            const SizedBox(width: 10),
            Expanded(child: Text(title, style: Theme.of(context).textTheme.titleSmall?.copyWith(fontWeight: FontWeight.w700))),
          ]),
          if (message != null && message!.isNotEmpty) ...[const SizedBox(height: 8), Text(message!)],
          if (child != null) ...[const SizedBox(height: 12), child!],
        ],
      ),
    );
  }
}

/// A row of small facts under a title ("Acme · Remote · Full time").
class MetaLine extends StatelessWidget {
  final List<String> parts;
  const MetaLine(this.parts, {super.key});

  @override
  Widget build(BuildContext context) {
    final text = parts.where((p) => p.trim().isNotEmpty).join(' · ');
    if (text.isEmpty) return const SizedBox.shrink();
    final theme = Theme.of(context);
    return Text(text, maxLines: 2, overflow: TextOverflow.ellipsis, style: theme.textTheme.bodySmall?.copyWith(color: theme.colorScheme.onSurfaceVariant));
  }
}

/// Shows the outcome of a desktop request in a SnackBar. Returns whether it
/// succeeded.
bool showOutcome(BuildContext context, RelayResponse resp, {String? success}) {
  final messenger = ScaffoldMessenger.maybeOf(context);
  if (resp.ok) {
    final data = resp.data;
    // The desktop queued it behind what it's doing: say that, not "started".
    final queued = data is Map && data['queued'] == true;
    final text = queued ? queuedMessage(data) : success;
    if (text != null) messenger?.showSnackBar(SnackBar(content: Text(text)));
    return true;
  }
  messenger?.showSnackBar(SnackBar(content: Text(friendlyError(resp)), duration: const Duration(seconds: 6)));
  return false;
}

String queuedMessage(Map data) {
  final position = data['position'];
  final title = data['title'];
  final where = position is num && position > 0 ? ' (#${position.toInt()})' : '';
  final what = title is String && title.isNotEmpty ? ': $title' : '';
  return 'Queued on your desktop$where$what. It starts when the agent is free.';
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
Future<String?> textConfirmDialog(BuildContext context, {required String title, required String message, required String label, String confirm = 'Confirm', bool required = false, String? hint, int maxLines = 3, TextInputType? keyboardType}) {
  return showDialog<String>(
    context: context,
    builder: (ctx) => _TextConfirmDialog(title: title, message: message, label: label, confirm: confirm, required: required, hint: hint, maxLines: maxLines, keyboardType: keyboardType),
  );
}

class _TextConfirmDialog extends StatefulWidget {
  final String title, message, label, confirm;
  final bool required;
  final String? hint;
  final int maxLines;
  final TextInputType? keyboardType;
  const _TextConfirmDialog({required this.title, required this.message, required this.label, required this.confirm, required this.required, this.hint, this.maxLines = 3, this.keyboardType});

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
            TextField(
              controller: _controller,
              decoration: InputDecoration(labelText: widget.label, hintText: widget.hint),
              maxLines: widget.maxLines,
              minLines: 1,
              keyboardType: widget.keyboardType,
              autofocus: true,
              onChanged: (_) => setState(() {}),
            ),
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
