import 'package:flutter/material.dart';

import '../models/application.dart';

/// "Got an interview call": a bottom sheet to record when, how and what to
/// prepare. Returns the payload for `record_interview`, or null when
/// dismissed. Everything is optional.
Future<Map<String, dynamic>?> showInterviewSheet(BuildContext context, {required String company, bool another = false}) {
  return showModalBottomSheet<Map<String, dynamic>>(
    context: context,
    isScrollControlled: true,
    useSafeArea: true,
    builder: (_) => _InterviewSheet(company: company, another: another),
  );
}

class _InterviewSheet extends StatefulWidget {
  final String company;
  final bool another;
  const _InterviewSheet({required this.company, required this.another});

  @override
  State<_InterviewSheet> createState() => _InterviewSheetState();
}

class _InterviewSheetState extends State<_InterviewSheet> {
  DateTime? _date;
  TimeOfDay? _time;
  String _mode = '';
  final _round = TextEditingController();
  final _contact = TextEditingController();
  final _notes = TextEditingController();

  @override
  void dispose() {
    _round.dispose();
    _contact.dispose();
    _notes.dispose();
    super.dispose();
  }

  DateTime? get _scheduledAt {
    final d = _date;
    if (d == null) return null;
    final t = _time ?? const TimeOfDay(hour: 10, minute: 0);
    return DateTime(d.year, d.month, d.day, t.hour, t.minute);
  }

  Future<void> _pickDate() async {
    final now = DateTime.now();
    final picked = await showDatePicker(context: context, initialDate: _date ?? now, firstDate: now.subtract(const Duration(days: 30)), lastDate: now.add(const Duration(days: 365)));
    if (picked != null) setState(() => _date = picked);
  }

  Future<void> _pickTime() async {
    final picked = await showTimePicker(context: context, initialTime: _time ?? const TimeOfDay(hour: 10, minute: 0));
    if (picked != null) setState(() => _time = picked);
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final when = _scheduledAt;
    return Padding(
      padding: EdgeInsets.fromLTRB(20, 16, 20, 20 + MediaQuery.viewInsetsOf(context).bottom),
      child: SingleChildScrollView(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Center(child: Container(width: 36, height: 4, decoration: BoxDecoration(color: theme.colorScheme.outlineVariant, borderRadius: BorderRadius.circular(2)))),
            const SizedBox(height: 16),
            Text(widget.another ? 'Add another interview' : 'Got an interview call?', style: theme.textTheme.titleLarge?.copyWith(fontWeight: FontWeight.w800)),
            const SizedBox(height: 4),
            Text('Record the interview ${widget.company} invited you to. Everything is optional; add what you know.', style: theme.textTheme.bodySmall?.copyWith(color: theme.colorScheme.onSurfaceVariant)),
            const SizedBox(height: 16),
            Row(children: [
              Expanded(
                child: OutlinedButton.icon(
                  onPressed: _pickDate,
                  icon: const Icon(Icons.calendar_today, size: 18),
                  label: Text(_date == null ? 'Date' : MaterialLocalizations.of(context).formatShortDate(_date!)),
                ),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: OutlinedButton.icon(
                  onPressed: _pickTime,
                  icon: const Icon(Icons.schedule, size: 18),
                  label: Text(_time == null ? 'Time' : _time!.format(context)),
                ),
              ),
            ]),
            if (when != null)
              Padding(
                padding: const EdgeInsets.only(top: 6),
                child: Text('${MaterialLocalizations.of(context).formatFullDate(when)} at ${TimeOfDay.fromDateTime(when).format(context)}', style: theme.textTheme.bodySmall?.copyWith(color: theme.colorScheme.primary, fontWeight: FontWeight.w600)),
              ),
            const SizedBox(height: 14),
            Text('How', style: theme.textTheme.labelLarge),
            const SizedBox(height: 6),
            Wrap(spacing: 8, runSpacing: 8, children: [
              for (final entry in interviewModeLabels.entries)
                ChoiceChip(label: Text(entry.value), selected: _mode == entry.key, onSelected: (on) => setState(() => _mode = on ? entry.key : '')),
            ]),
            const SizedBox(height: 14),
            TextField(controller: _round, decoration: const InputDecoration(labelText: 'Round', hintText: 'e.g. HR screening, Technical round 1'), textCapitalization: TextCapitalization.sentences),
            const SizedBox(height: 12),
            TextField(controller: _contact, decoration: const InputDecoration(labelText: 'Who called', hintText: 'Name or team'), textCapitalization: TextCapitalization.words),
            const SizedBox(height: 12),
            TextField(controller: _notes, decoration: const InputDecoration(labelText: 'Notes', hintText: 'What they said, what to prepare'), minLines: 2, maxLines: 5, textCapitalization: TextCapitalization.sentences),
            const SizedBox(height: 18),
            FilledButton.icon(
              onPressed: () => Navigator.of(context).pop({
                if (when != null) 'scheduledAt': when.toUtc().toIso8601String(),
                'mode': _mode,
                'round': _round.text.trim(),
                'contact': _contact.text.trim(),
                'notes': _notes.text.trim(),
              }),
              icon: const Icon(Icons.event_available),
              label: const Text('Record interview'),
            ),
          ],
        ),
      ),
    );
  }
}
