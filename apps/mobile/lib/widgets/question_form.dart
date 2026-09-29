import 'package:flutter/material.dart';

import '../models/application.dart';

/// The answer for [q] as the desktop expects it, from the raw form value.
String answerValue(PendingQuestion q, String? raw) => (raw ?? '').trim();

/// Builds the `{question, answer}` list to send, skipping blanks (the
/// desktop ignores empty answers and keeps those questions pending).
List<ApplicationAnswer> collectAnswers(List<PendingQuestion> questions, Map<String, String> values) => [
      for (final q in questions)
        if (answerValue(q, values[q.key]).isNotEmpty) ApplicationAnswer(question: q.question, answer: answerValue(q, values[q.key])),
    ];

/// Which required questions are still blank.
List<PendingQuestion> missingRequired(List<PendingQuestion> questions, Map<String, String> values) =>
    questions.where((q) => q.required && answerValue(q, values[q.key]).isEmpty).toList();

/// Answers the questions the desktop agent stopped on (an application form
/// or a job-site profile): one field per question, shaped by its
/// `fieldType` (text, number, date, select, radio, textarea, checkbox, ...),
/// with required ones marked. The desktop saves every answer for reuse.
class QuestionForm extends StatefulWidget {
  final List<PendingQuestion> questions;

  /// Sends the answers; resolves true when the desktop accepted them.
  final Future<bool> Function(List<ApplicationAnswer> answers) onSubmit;

  /// Null when the desktop can be reached; else why it can't.
  final String? disabledReason;
  final String submitLabel;

  const QuestionForm({super.key, required this.questions, required this.onSubmit, this.disabledReason, this.submitLabel = 'Send answers'});

  @override
  State<QuestionForm> createState() => _QuestionFormState();
}

class _QuestionFormState extends State<QuestionForm> {
  final _formKey = GlobalKey<FormState>();
  final _values = <String, String>{};
  final _controllers = <String, TextEditingController>{};
  bool _sending = false;

  @override
  void dispose() {
    for (final c in _controllers.values) {
      c.dispose();
    }
    super.dispose();
  }

  TextEditingController _controller(PendingQuestion q) => _controllers.putIfAbsent(q.key, () => TextEditingController(text: _values[q.key] ?? ''));

  String? _requiredValidator(PendingQuestion q, String? v) => q.required && (v == null || v.trim().isEmpty) ? 'Required' : null;

  Future<void> _submit() async {
    if (!_formKey.currentState!.validate()) return;
    final answers = collectAnswers(widget.questions, _values);
    if (answers.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(const SnackBar(content: Text('Answer at least one question.')));
      return;
    }
    setState(() => _sending = true);
    await widget.onSubmit(answers);
    if (mounted) setState(() => _sending = false);
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Form(
      key: _formKey,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          for (final q in widget.questions)
            Padding(
              padding: const EdgeInsets.only(bottom: 16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Text.rich(
                    TextSpan(children: [
                      TextSpan(text: q.question, style: theme.textTheme.bodyLarge?.copyWith(fontWeight: FontWeight.w600)),
                      if (q.required) TextSpan(text: ' *', style: TextStyle(color: theme.colorScheme.error, fontWeight: FontWeight.w700)),
                    ]),
                  ),
                  if (q.context.isNotEmpty) ...[
                    const SizedBox(height: 2),
                    Text(q.context, style: theme.textTheme.bodySmall?.copyWith(color: theme.colorScheme.onSurfaceVariant)),
                  ],
                  const SizedBox(height: 8),
                  _field(q),
                ],
              ),
            ),
          Text('* required. Your answers are saved to Additional details so the agent can reuse them next time.',
              style: theme.textTheme.bodySmall?.copyWith(color: theme.colorScheme.onSurfaceVariant)),
          if (widget.disabledReason != null) ...[
            const SizedBox(height: 8),
            Text(widget.disabledReason!, style: TextStyle(color: theme.colorScheme.error)),
          ],
          const SizedBox(height: 12),
          FilledButton.icon(
            onPressed: _sending || widget.disabledReason != null ? null : _submit,
            icon: _sending ? const SizedBox(width: 18, height: 18, child: CircularProgressIndicator(strokeWidth: 2)) : const Icon(Icons.send),
            label: Text(widget.submitLabel),
          ),
        ],
      ),
    );
  }

  Widget _field(PendingQuestion q) {
    final type = q.fieldType;
    final hasOptions = q.options.isNotEmpty;
    if ((type == 'select' || type == 'dropdown') && hasOptions) return _select(q);
    if (type == 'radio' && hasOptions) return _radio(q, q.options);
    if (type == 'checkbox' || type == 'boolean' || type == 'yesno') return hasOptions ? _multi(q) : _radio(q, const ['Yes', 'No']);
    if (type == 'date') return _date(q);
    if (hasOptions && type != 'textarea') return _select(q);
    final multiline = type == 'textarea';
    return TextFormField(
      controller: _controller(q),
      keyboardType: switch (type) {
        'number' => const TextInputType.numberWithOptions(decimal: true),
        'email' => TextInputType.emailAddress,
        'tel' || 'phone' => TextInputType.phone,
        'url' => TextInputType.url,
        _ => multiline ? TextInputType.multiline : TextInputType.text,
      },
      minLines: multiline ? 3 : 1,
      maxLines: multiline ? 8 : 1,
      decoration: const InputDecoration(isDense: true),
      validator: (v) {
        final missing = _requiredValidator(q, v);
        if (missing != null) return missing;
        if (type == 'number' && v != null && v.trim().isNotEmpty && num.tryParse(v.trim()) == null) return 'Enter a number';
        return null;
      },
      onChanged: (v) => _values[q.key] = v,
    );
  }

  Widget _select(PendingQuestion q) => DropdownButtonFormField<String>(
        initialValue: q.options.contains(_values[q.key]) ? _values[q.key] : null,
        isExpanded: true,
        decoration: const InputDecoration(isDense: true, hintText: 'Choose…'),
        items: q.options.map((o) => DropdownMenuItem(value: o, child: Text(o, overflow: TextOverflow.ellipsis))).toList(),
        onChanged: (v) => setState(() => _values[q.key] = v ?? ''),
        validator: (v) => _requiredValidator(q, v),
      );

  Widget _radio(PendingQuestion q, List<String> options) => FormField<String>(
        initialValue: _values[q.key],
        validator: (_) => _requiredValidator(q, _values[q.key]),
        builder: (state) => Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            RadioGroup<String>(
              groupValue: _values[q.key],
              onChanged: (v) {
                setState(() => _values[q.key] = v ?? '');
                state.didChange(v);
              },
              child: Column(children: [
                for (final o in options) RadioListTile<String>(value: o, title: Text(o), dense: true, contentPadding: EdgeInsets.zero),
              ]),
            ),
            if (state.hasError) Text(state.errorText!, style: TextStyle(color: Theme.of(context).colorScheme.error, fontSize: 12)),
          ],
        ),
      );

  Widget _multi(PendingQuestion q) {
    final selected = (_values[q.key] ?? '').split(', ').where((s) => s.isNotEmpty).toSet();
    return FormField<String>(
      validator: (_) => _requiredValidator(q, _values[q.key]),
      builder: (state) => Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Wrap(spacing: 6, runSpacing: 6, children: [
            for (final o in q.options)
              FilterChip(
                label: Text(o),
                selected: selected.contains(o),
                onSelected: (on) {
                  final next = {...selected};
                  on ? next.add(o) : next.remove(o);
                  setState(() => _values[q.key] = q.options.where(next.contains).join(', '));
                  state.didChange(_values[q.key]);
                },
              ),
          ]),
          if (state.hasError) Text(state.errorText!, style: TextStyle(color: Theme.of(context).colorScheme.error, fontSize: 12)),
        ],
      ),
    );
  }

  Widget _date(PendingQuestion q) {
    final controller = _controller(q);
    return TextFormField(
      controller: controller,
      readOnly: true,
      decoration: const InputDecoration(isDense: true, hintText: 'YYYY-MM-DD', suffixIcon: Icon(Icons.calendar_today, size: 18)),
      validator: (v) => _requiredValidator(q, v),
      onTap: () async {
        final now = DateTime.now();
        final picked = await showDatePicker(context: context, initialDate: DateTime.tryParse(controller.text) ?? now, firstDate: DateTime(now.year - 80), lastDate: DateTime(now.year + 10));
        if (picked == null) return;
        final text = '${picked.year.toString().padLeft(4, '0')}-${picked.month.toString().padLeft(2, '0')}-${picked.day.toString().padLeft(2, '0')}';
        controller.text = text;
        setState(() => _values[q.key] = text);
      },
    );
  }
}
