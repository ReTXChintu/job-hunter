import 'package:flutter/material.dart';

import '../logic/filters.dart';

/// The location filter as a chip: "Any location" or the chosen place with a
/// clear button. Tapping opens a sheet listing the places already in the
/// data (with counts) and a box to type any other. Searches only what's on
/// the phone, never the web.
class LocationChip extends StatelessWidget {
  final String value;
  final List<LocationOption> options;
  final ValueChanged<String> onChanged;
  const LocationChip({super.key, required this.value, required this.options, required this.onChanged});

  Future<void> _pick(BuildContext context) async {
    final picked = await showModalBottomSheet<String>(
      context: context,
      isScrollControlled: true,
      useSafeArea: true,
      builder: (_) => _LocationSheet(current: value, options: options),
    );
    if (picked != null) onChanged(picked);
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final active = value.isNotEmpty;
    return InputChip(
      avatar: Icon(Icons.place_outlined, size: 16, color: active ? theme.colorScheme.onPrimaryContainer : theme.colorScheme.onSurfaceVariant),
      label: Text(active ? value : 'Any location'),
      selected: active,
      showCheckmark: false,
      onPressed: () => _pick(context),
      onDeleted: active ? () => onChanged('') : null,
      deleteIcon: const Icon(Icons.close, size: 16),
      deleteButtonTooltipMessage: 'Any location',
    );
  }
}

class _LocationSheet extends StatefulWidget {
  final String current;
  final List<LocationOption> options;
  const _LocationSheet({required this.current, required this.options});

  @override
  State<_LocationSheet> createState() => _LocationSheetState();
}

class _LocationSheetState extends State<_LocationSheet> {
  late final _typed = TextEditingController(text: widget.current);

  @override
  void dispose() {
    _typed.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final q = _typed.text.trim().toLowerCase();
    final shown = widget.options.where((o) => q.isEmpty || o.label.toLowerCase().contains(q)).toList();
    return DraggableScrollableSheet(
      expand: false,
      initialChildSize: 0.6,
      maxChildSize: 0.9,
      builder: (context, scroll) => Column(
        children: [
          Padding(
            padding: const EdgeInsets.fromLTRB(20, 14, 20, 8),
            child: Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
              Center(child: Container(width: 36, height: 4, decoration: BoxDecoration(color: theme.colorScheme.outlineVariant, borderRadius: BorderRadius.circular(2)))),
              const SizedBox(height: 14),
              Text('Location', style: theme.textTheme.titleLarge?.copyWith(fontWeight: FontWeight.w800)),
              Text('Only looks through the jobs already found, never the web.', style: theme.textTheme.bodySmall?.copyWith(color: theme.colorScheme.onSurfaceVariant)),
              const SizedBox(height: 12),
              TextField(
                controller: _typed,
                autofocus: widget.options.isEmpty,
                textInputAction: TextInputAction.search,
                decoration: InputDecoration(
                  prefixIcon: const Icon(Icons.search, size: 20),
                  hintText: 'Type a city, e.g. Mumbai',
                  suffixIcon: _typed.text.isEmpty ? null : IconButton(icon: const Icon(Icons.clear, size: 18), onPressed: () => setState(_typed.clear)),
                ),
                onChanged: (_) => setState(() {}),
                onSubmitted: (v) => Navigator.of(context).pop(v.trim()),
              ),
            ]),
          ),
          Expanded(
            child: ListView(
              controller: scroll,
              padding: const EdgeInsets.only(bottom: 16),
              children: [
                ListTile(
                  leading: const Icon(Icons.public),
                  title: const Text('Any location'),
                  selected: widget.current.isEmpty,
                  onTap: () => Navigator.of(context).pop(''),
                ),
                if (q.isNotEmpty && !shown.any((o) => o.label.toLowerCase() == q))
                  ListTile(
                    leading: const Icon(Icons.search),
                    title: Text('Search "${_typed.text.trim()}"'),
                    subtitle: const Text('Anywhere the location mentions it'),
                    onTap: () => Navigator.of(context).pop(_typed.text.trim()),
                  ),
                for (final o in shown)
                  ListTile(
                    leading: Icon(o.label == 'Remote' ? Icons.home_work_outlined : Icons.place_outlined),
                    title: Text(o.label),
                    trailing: Text('${o.count}', style: theme.textTheme.labelLarge?.copyWith(color: theme.colorScheme.onSurfaceVariant)),
                    selected: widget.current.toLowerCase() == o.label.toLowerCase(),
                    onTap: () => Navigator.of(context).pop(o.label),
                  ),
                if (shown.isEmpty && q.isEmpty)
                  Padding(
                    padding: const EdgeInsets.all(20),
                    child: Text('No locations yet. Jobs the desktop finds bring their places here.', textAlign: TextAlign.center, style: theme.textTheme.bodySmall?.copyWith(color: theme.colorScheme.onSurfaceVariant)),
                  ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
