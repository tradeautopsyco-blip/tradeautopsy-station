import SwiftUI

/// Tags + playbooks on Plan (`founder-backlog.md` §5). Conditions stay on `declaration_id` elsewhere.
struct BarPlanAuthoringExtras: View {
    @ObservedObject var viewModel: NotchViewModel
    @State private var tagDraft: String = ""
    @State private var playbookTitle: String = ""
    @State private var playbookBody: String = ""

    private var draftDeclarationId: String {
        viewModel.barPublishedDeclarationId?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Tags & playbooks")
                .font(BarDS.bodyFont(11, weight: .semibold))
                .foregroundColor(BarDS.Text.section)
                .textCase(.uppercase)

            if draftDeclarationId.isEmpty {
                Text("Tags attach after your first Confirm (declaration id). Playbooks are local until Console ships authoring.")
                    .font(BarDS.monoFont(10, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                    .fixedSize(horizontal: false, vertical: true)
            } else {
                tagEditor
            }

            playbookSection
        }
    }

    private var tagEditor: some View {
        let id = draftDeclarationId
        let tags = BarDeclarationTagsStore.shared.tags(for: id)
        return VStack(alignment: .leading, spacing: 6) {
            FlowLayout(tags: tags) { tag in
                Text(tag)
                    .font(BarDS.monoFont(10, weight: .medium))
                    .padding(.horizontal, 8)
                    .padding(.vertical, 4)
                    .background(Color.white.opacity(0.06))
                    .clipShape(Capsule())
            }
            HStack {
                BarInputField(placeholder: "Add tag", text: $tagDraft)
                Button("Add") {
                    var next = tags
                    let t = tagDraft.trimmingCharacters(in: .whitespacesAndNewlines)
                    guard !t.isEmpty else { return }
                    next.append(t)
                    BarDeclarationTagsStore.shared.setTags(next, for: id)
                    tagDraft = ""
                }
                .buttonStyle(.plain)
                .font(BarDS.bodyFont(11, weight: .semibold))
                .foregroundColor(BarDS.Accent.teal)
            }
        }
    }

    private var playbookSection: some View {
        VStack(alignment: .leading, spacing: 6) {
            ForEach(BarPlaybookStore.load()) { pb in
                Text(pb.title)
                    .font(BarDS.bodyFont(11, weight: .semibold))
                Text(pb.body)
                    .font(BarDS.monoFont(10, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
            }
            BarInputField(placeholder: "Playbook title", text: $playbookTitle)
            BarInputField(placeholder: "Playbook note", text: $playbookBody)
            Button("Save playbook locally") {
                let t = playbookTitle.trimmingCharacters(in: .whitespacesAndNewlines)
                let b = playbookBody.trimmingCharacters(in: .whitespacesAndNewlines)
                guard !t.isEmpty, !b.isEmpty else { return }
                _ = BarPlaybookStore.upsert(title: t, body: b)
                playbookTitle = ""
                playbookBody = ""
            }
            .buttonStyle(.plain)
            .font(BarDS.bodyFont(11, weight: .semibold))
            .foregroundColor(BarDS.Accent.teal)
        }
    }
}

/// Minimal tag flow layout.
private struct FlowLayout: View {
    let tags: [String]
    let content: (String) -> AnyView

    init(tags: [String], @ViewBuilder content: @escaping (String) -> some View) {
        self.tags = tags
        self.content = { AnyView(content($0)) }
    }

    var body: some View {
        HStack(spacing: 6) {
            ForEach(tags, id: \.self) { tag in
                content(tag)
            }
        }
    }
}
