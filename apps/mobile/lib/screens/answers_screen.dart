import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../logic/labels.dart';
import '../models/profiles.dart';
import '../state/connection_controller.dart';
import '../state/profiles_controller.dart';
import '../widgets/common.dart';
import '../widgets/empty_state.dart';
import '../widgets/presence_banner.dart';

/// More > Additional details: the answers the agent reuses on application
/// forms (notice period, current CTC, ...). Editing and adding go to the
/// desktop; while it's offline the list is read-only from the server.
class AnswersScreen extends StatefulWidget {
  const AnswersScreen({super.key});

  @override
  State<AnswersScreen> createState() => _AnswersScreenState();
}

class _AnswersScreenState extends State<AnswersScreen> {
  final _search = TextEditingController();

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => context.read<AnswersController>().refresh());
  }

  @override
  void dispose() {
    _search.dispose();
    super.dispose();
  }

  Future<void> _edit(AnswerRecord? record) async {
    final result = await showDialog<(String, String)>(context: context, builder: (_) => _AnswerDialog(record: record));
    if (result == null || !mounted) return;
    final resp = await context.read<AnswersController>().save(id: record?.id, question: result.$1, answer: result.$2);
    if (mounted) showOutcome(context, resp, success: 'Saved.');
  }

  @override
  Widget build(BuildContext context) {
    final answers = context.watch<AnswersController>();
    final connection = context.watch<ConnectionController>();
    final canEdit = connection.desktopReachable;
    final q = _search.text.trim().toLowerCase();
    final rows = answers.items.where((a) => q.isEmpty || a.question.toLowerCase().contains(q) || a.answer.toLowerCase().contains(q)).toList();

    Widget body;
    if (answers.items.isEmpty && answers.loading) {
      body = const Center(child: CircularProgressIndicator());
    } else if (rows.isEmpty) {
      body = RefreshIndicator(
        onRefresh: answers.refresh,
        child: LayoutBuilder(
          builder: (context, c) => ListView(physics: const AlwaysScrollableScrollPhysics(), children: [
            SizedBox(
              height: c.maxHeight,
              child: EmptyState(
                icon: answers.error != null ? Icons.error_outline : Icons.question_answer_outlined,
                title: answers.error != null ? 'Could not load saved answers' : (q.isEmpty ? 'No saved answers yet' : 'No matches'),
                message: answers.error ?? (q.isEmpty ? 'Answers you give on application forms are saved here for the agent to reuse.' : null),
              ),
            ),
          ]),
        ),
      );
    } else {
      body = RefreshIndicator(
        onRefresh: answers.refresh,
        child: ListView.separated(
          physics: const AlwaysScrollableScrollPhysics(),
          padding: const EdgeInsets.only(bottom: 88),
          itemCount: rows.length,
          separatorBuilder: (context, i) => const Divider(height: 1),
          itemBuilder: (context, i) {
            final a = rows[i];
            return ListTile(
              title: Text(a.question, style: const TextStyle(fontWeight: FontWeight.w600)),
              subtitle: Text(a.answer.isEmpty ? '(no answer)' : a.answer),
              trailing: canEdit ? const Icon(Icons.edit_outlined, size: 20) : null,
              onTap: canEdit ? () => _edit(a) : null,
            );
          },
        ),
      );
    }

    return Scaffold(
      appBar: AppBar(title: const Text('Additional details')),
      floatingActionButton: canEdit ? FloatingActionButton.extended(onPressed: () => _edit(null), icon: const Icon(Icons.add), label: const Text('Add')) : null,
      body: Column(children: [
        const PresenceBanner(),
        DataSourceBanner(answers.source),
        if (!canEdit && answers.items.isNotEmpty)
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 8, 16, 0),
            child: Text('Read-only: ${connection.connected ? desktopOfflineMessage : notConnectedMessage}', style: Theme.of(context).textTheme.bodySmall),
          ),
        Padding(
          padding: const EdgeInsets.fromLTRB(12, 10, 12, 6),
          child: TextField(
            controller: _search,
            onChanged: (_) => setState(() {}),
            decoration: const InputDecoration(isDense: true, prefixIcon: Icon(Icons.search), hintText: 'Search questions and answers'),
          ),
        ),
        Expanded(child: body),
      ]),
    );
  }
}

class _AnswerDialog extends StatefulWidget {
  final AnswerRecord? record;
  const _AnswerDialog({this.record});

  @override
  State<_AnswerDialog> createState() => _AnswerDialogState();
}

class _AnswerDialogState extends State<_AnswerDialog> {
  late final _question = TextEditingController(text: widget.record?.question ?? '');
  late final _answer = TextEditingController(text: widget.record?.answer ?? '');

  @override
  void dispose() {
    _question.dispose();
    _answer.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final valid = _question.text.trim().isNotEmpty && _answer.text.trim().isNotEmpty;
    return AlertDialog(
      title: Text(widget.record == null ? 'New saved answer' : 'Edit answer'),
      content: SingleChildScrollView(
        child: Column(mainAxisSize: MainAxisSize.min, children: [
          TextField(controller: _question, decoration: const InputDecoration(labelText: 'Question'), minLines: 1, maxLines: 3, onChanged: (_) => setState(() {})),
          const SizedBox(height: 12),
          TextField(controller: _answer, decoration: const InputDecoration(labelText: 'Answer'), minLines: 1, maxLines: 6, onChanged: (_) => setState(() {})),
        ]),
      ),
      actions: [
        TextButton(onPressed: () => Navigator.of(context).pop(), child: const Text('Cancel')),
        FilledButton(onPressed: valid ? () => Navigator.of(context).pop((_question.text.trim(), _answer.text.trim())) : null, child: const Text('Save')),
      ],
    );
  }
}
