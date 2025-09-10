import SwiftUI
import UIKit
import Security
import Dialog

struct SettingsView: View {
    @ObservedObject var viewModel: InboxViewModel
    let dismiss: () -> Void

    @AppStorage("DIALOG_RELAY") private var relayUrl: String = "wss://relay.damus.io"
    @State private var showingQR = false
    @State private var showSignOutAlert = false
    @State private var ephemeralNpub: String = ""

    var body: some View {
        NavigationStack {
            Form {
                Section("Relay") {
                    TextField("Relay URL", text: $relayUrl)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                        .keyboardType(.URL)
                        .onSubmit { viewModel.connectRelay(relayUrl) }
                        .onChange(of: relayUrl) { _, new in
                            // Connect on change (debounced by user typing)
                            viewModel.connectRelay(new)
                        }
                }

                Section("Account") {
                    HStack {
                        Text("npub")
                        Spacer()
                        Text(npubForCurrent())
                            .font(.system(.footnote, design: .monospaced))
                            .foregroundStyle(.secondary)
                            .lineLimit(1)
                            .contextMenu { Button("Copy") { UIPasteboard.general.string = npubForCurrent() } }
                    }
                    Button {
                        showingQR = true
                    } label: {
                        Label("Show nsec as QR", systemImage: "qrcode")
                    }
                    .alert("Your nsec is secret. Anyone with it can access your notes.", isPresented: $showingQR) {
                        Button("Cancel", role: .cancel) {}
                        Button("I understand") { presentQR() }
                    }
                }

                Section {
                    Button(role: .destructive) {
                        showSignOutAlert = true
                    } label: {
                        Label("Sign out (wipe local data)", systemImage: "trash")
                    }
                } footer: {
                    Text("Signing out deletes local cache and Keychain entry; you will need to re-enter your nsec. App restart may be required.")
                }
            }
            .navigationTitle("Settings")
            .toolbar { ToolbarItem(placement: .topBarTrailing) { Button("Done") { dismiss() } } }
            .confirmationDialog("Sign out? This deletes local data.", isPresented: $showSignOutAlert) {
                Button("Sign out", role: .destructive) {
                    // Clear data dir and Keychain entry
                    viewModel.clearData()
                    KeychainService.delete(key: "nsec")
                    dismiss()
                }
                Button("Cancel", role: .cancel) {}
            }
        }
    }

    private func npubForCurrent() -> String {
        if !ephemeralNpub.isEmpty { return ephemeralNpub }
        ephemeralNpub = viewModel.deriveNpub(from: viewModel.nsecInUse)
        return ephemeralNpub
    }

    private func presentQR() {
        guard let img = QRGenerator.generate(from: viewModel.nsecInUse) else { return }
        let vc = UIActivityViewController(activityItems: [img], applicationActivities: nil)
        UIApplication.shared.connectedScenes
            .compactMap { $0 as? UIWindowScene }
            .first?.keyWindow?.rootViewController?.present(vc, animated: true)
    }
}

// MARK: - QR
enum QRGenerator {
    static func generate(from string: String) -> UIImage? {
        guard let data = string.data(using: .utf8) else { return nil }
        guard let filter = CIFilter(name: "CIQRCodeGenerator") else { return nil }
        filter.setValue(data, forKey: "inputMessage")
        filter.setValue("M", forKey: "inputCorrectionLevel")
        guard let output = filter.outputImage else { return nil }
        let scaleX: CGFloat = 6
        let scaleY: CGFloat = 6
        let transformed = output.transformed(by: CGAffineTransform(scaleX: scaleX, y: scaleY))
        return UIImage(ciImage: transformed)
    }
}
