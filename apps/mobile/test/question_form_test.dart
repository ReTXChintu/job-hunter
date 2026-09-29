import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:job_hunter_mobile/models/application.dart';
import 'package:job_hunter_mobile/widgets/question_form.dart';

const _questions = [
  PendingQuestion(id: 'q1', question: 'Years of Flutter?', fieldType: 'number', required: true),
  PendingQuestion(id: 'q2', question: 'Willing to relocate?', fieldType: 'radio', options: ['Yes', 'No']),
  PendingQuestion(id: 'q3', question: 'Anything else?', fieldType: 'textarea'),
];

void main() {
  test('blank answers are left out and required blanks are reported', () {
    final values = {'q1': ' 4 ', 'q2': '', 'q3': 'No'};
    final answers = collectAnswers(_questions, values);
    expect(answers.map((a) => a.toJson()), [
      {'question': 'Years of Flutter?', 'answer': '4'},
      {'question': 'Anything else?', 'answer': 'No'},
    ]);
    expect(missingRequired(_questions, {'q2': 'Yes'}).map((q) => q.id), ['q1']);
  });

  testWidgets('the form marks required fields, validates, and submits the answers', (tester) async {
    List<ApplicationAnswer>? sent;
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(
        body: SingleChildScrollView(
          child: QuestionForm(
            questions: _questions,
            onSubmit: (answers) async {
              sent = answers;
              return true;
            },
          ),
        ),
      ),
    ));

    expect(find.text('Years of Flutter? *'), findsOneWidget, reason: 'required questions are starred');
    expect(find.text('Willing to relocate?'), findsOneWidget);
    expect(find.byType(RadioListTile<String>), findsNWidgets(2));

    // Submitting with the required field empty is refused.
    await tester.tap(find.text('Send answers'));
    await tester.pump();
    expect(find.text('Required'), findsOneWidget);
    expect(sent, isNull);

    await tester.enterText(find.byType(TextFormField).first, 'abc');
    await tester.tap(find.text('Send answers'));
    await tester.pump();
    expect(find.text('Enter a number'), findsOneWidget);

    await tester.enterText(find.byType(TextFormField).first, '4');
    await tester.tap(find.text('Yes'));
    await tester.tap(find.text('Send answers'));
    await tester.pumpAndSettle();
    expect(sent!.map((a) => a.toJson()), [
      {'question': 'Years of Flutter?', 'answer': '4'},
      {'question': 'Willing to relocate?', 'answer': 'Yes'},
    ]);
  });

  testWidgets('the form is disabled with a reason when the desktop is offline', (tester) async {
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(
        body: SingleChildScrollView(child: QuestionForm(questions: _questions, disabledReason: 'Your desktop is offline.', onSubmit: (_) async => true)),
      ),
    ));
    expect(find.text('Your desktop is offline.'), findsOneWidget);
    final button = tester.widget<ButtonStyleButton>(find.ancestor(of: find.text('Send answers'), matching: find.byWidgetPredicate((w) => w is ButtonStyleButton)));
    expect(button.onPressed, isNull);
  });
}
