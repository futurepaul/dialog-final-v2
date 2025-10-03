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
    @State private var npubCopied = false
    @State private var nsecCopied = false
    @State private var showingImportSheet = false
    @State private var importNsec = ""
    @State private var showImportConfirm = false
    @State private var importError: String?

    var body: some View {
        NavigationStack {
            Form {
                Section("Relay") {
                    HStack {
                        TextField("Relay URL", text: $relayUrl)
                            .textInputAutocapitalization(.never)
                            .autocorrectionDisabled()
                            .keyboardType(.URL)
                            .onSubmit { viewModel.connectRelay(relayUrl) }
                        Button("Connect") { viewModel.connectRelay(relayUrl) }
                            .buttonStyle(.bordered)
                            .controlSize(.small)
                    }
                }

                Section("Account") {
                    VStack(alignment: .leading, spacing: 8) {
                        Text("npub")
                        HStack {
                            Text(viewModel.npub)
                                .font(.system(.footnote, design: .monospaced))
                                .foregroundStyle(.secondary)
                                .lineLimit(1)
                                .truncationMode(.middle)
                            Spacer()
                            Button {
                                UIPasteboard.general.string = viewModel.npub
                                npubCopied = true
                                DispatchQueue.main.asyncAfter(deadline: .now() + 1.2) { npubCopied = false }
                            } label: {
                                Label(npubCopied ? "Copied" : "Copy", systemImage: npubCopied ? "checkmark" : "doc.on.doc")
                            }
                            .buttonStyle(.bordered)
                            .controlSize(.small)
                        }
                    }
                    Button {
                        showingQR = true
                    } label: {
                        Label("Show nsec as QR", systemImage: "qrcode")
                    }
                    Button {
                        UIPasteboard.general.string = viewModel.nsecInUse
                        nsecCopied = true
                        DispatchQueue.main.asyncAfter(deadline: .now() + 1.2) { nsecCopied = false }
                    } label: {
                        Label(nsecCopied ? "nsec copied" : "Copy nsec", systemImage: nsecCopied ? "checkmark" : "key")
                    }
                    Button {
                        importNsec = ""
                        importError = nil
                        showingImportSheet = true
                    } label: {
                        Label("Import nsec", systemImage: "arrow.triangle.2.circlepath.circle")
                    }
                }

                Section {
                    Button(role: .destructive) {
                        showSignOutAlert = true
                    } label: {
                        Label("Sign out (wipe local data)", systemImage: "trash")
                    }
                } header: {
                    Text("Danger Zone")
                } footer: {
                    Text("Signing out removes local cache and Keychain entry; you’ll need to re-enter your nsec to continue.")
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
            .sheet(isPresented: $showingImportSheet) {
                NavigationStack {
                    Form {
                        Section(
                            content: {
                                TextField("nsec1...", text: $importNsec)
                                    .textInputAutocapitalization(.never)
                                    .autocorrectionDisabled()
                                    .font(.system(.footnote, design: .monospaced))
                            },
                            footer: {
                                VStack(alignment: .leading, spacing: 4) {
                                    Text("Importing a new nsec will wipe local data for the current account.")
                                    Text("Make sure you've backed up the old nsec if you still need those notes.")
                                    if let error = importError {
                                        Text(error)
                                            .foregroundStyle(.red)
                                    }
                                }
                            }
                        )
                    }
                    .navigationTitle("Import nsec")
                    .toolbar {
                        ToolbarItem(placement: .topBarLeading) {
                            Button("Cancel") { showingImportSheet = false }
                        }
                        ToolbarItem(placement: .topBarTrailing) {
                            Button("Replace", role: .destructive) {
                                showImportConfirm = true
                            }
                            .disabled(!viewModel.validate(nsec: importNsec))
                        }
                    }
                    .alert("Replace current account?", isPresented: $showImportConfirm) {
                        Button("Replace", role: .destructive) {
                            if viewModel.importNsec(importNsec) {
                                showingImportSheet = false
                                dismiss()
                            } else {
                                importError = "Invalid nsec."
                            }
                        }
                        Button("Cancel", role: .cancel) {}
                    } message: {
                        Text("This will remove local data for the current account and replace it with the imported keys.")
                    }
                }
            }
        }
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
