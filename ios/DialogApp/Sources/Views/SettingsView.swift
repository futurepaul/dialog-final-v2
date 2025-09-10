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
    @State private var showingCopyToast = false

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
                    VStack(alignment: .leading, spacing: 8) {
                        Text("npub")
                        HStack {
                            Text(npubForCurrent())
                                .font(.system(.footnote, design: .monospaced))
                                .foregroundStyle(.secondary)
                                .lineLimit(1)
                                .truncationMode(.middle)
                            Spacer()
                            Button {
                                UIPasteboard.general.string = npubForCurrent()
                            } label: {
                                Label("Copy", systemImage: "doc.on.doc")
                            }
                            .buttonStyle(.borderless)
                        }
                    }
                    Button {
                        showingQR = true
                    } label: {
                        Label("Show nsec as QR", systemImage: "qrcode")
                    }
                    Button {
                        UIPasteboard.general.string = viewModel.nsecInUse
                    } label: {
                        Label("Copy nsec", systemImage: "key")
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
            .sheet(isPresented: $showingQR) {
                NsecQRSheet(nsec: viewModel.nsecInUse) { showingQR = false }
            }
        }
    }

    private func npubForCurrent() -> String {
        if !ephemeralNpub.isEmpty { return ephemeralNpub }
        ephemeralNpub = viewModel.deriveNpub(from: viewModel.nsecInUse)
        return ephemeralNpub
    }

    private func presentQR() { /* replaced with SwiftUI sheet below */ }
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

// MARK: - NsecQRSheet
struct NsecQRSheet: View {
    let nsec: String
    let onClose: () -> Void

    var body: some View {
        NavigationStack {
            VStack(spacing: 20) {
                if let img = QRGenerator.generate(from: nsec) {
                    Image(uiImage: img)
                        .interpolation(.none)
                        .resizable()
                        .scaledToFit()
                        .frame(maxWidth: 280, maxHeight: 280)
                        .padding()
                } else {
                    Text("Could not generate QR code")
                        .foregroundStyle(.secondary)
                }
                Text("Your nsec is secret. Anyone with it can access your notes.")
                    .multilineTextAlignment(.center)
                    .foregroundStyle(.secondary)
                    .padding(.horizontal)
                HStack {
                    Button {
                        UIPasteboard.general.string = nsec
                    } label: {
                        Label("Copy nsec", systemImage: "doc.on.doc")
                    }
                    .buttonStyle(.bordered)
                    Button {
                        let avc = UIActivityViewController(activityItems: [nsec], applicationActivities: nil)
                        UIApplication.shared.connectedScenes
                            .compactMap { $0 as? UIWindowScene }
                            .first?.keyWindow?.rootViewController?.present(avc, animated: true)
                    } label: {
                        Label("Share", systemImage: "square.and.arrow.up")
                    }
                    .buttonStyle(.bordered)
                }
                Spacer()
            }
            .padding()
            .navigationTitle("nsec QR")
            .toolbar { ToolbarItem(placement: .topBarTrailing) { Button("Done") { onClose() } } }
        }
    }
}
