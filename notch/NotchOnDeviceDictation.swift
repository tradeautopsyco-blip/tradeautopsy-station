import AVFoundation
import AppKit
import Foundation
import Security
import Speech

private final class ConsumeOnce: @unchecked Sendable {
    var done = false
}

// MARK: - On-device dictation (SFSpeechRecognizer + AVAudioEngine)

@MainActor
final class NotchOnDeviceDictationSession: NSObject {
    private let audioEngine = AVAudioEngine()
    private var recognitionRequest: SFSpeechAudioBufferRecognitionRequest?
    private var recognitionTask: SFSpeechRecognitionTask?
    private let speechRecognizer: SFSpeechRecognizer?

    private var composer = DictationTranscriptComposer()
    private var waveform = WaveformNineDotRing()
    private var silenceWatch = SilenceAutoStopWatch(
        rmsThreshold: 0.012,
        requiredQuietDuration: 30
    )

    private var recordingGate = DictationRecordingGate()
    private var reduceMotion = false

    private var converter: AVAudioConverter?
    private var targetFormat: AVAudioFormat?
    private var isEngineRunning = false
    private var configObserver: NSObjectProtocol?

    private var lastSilenceTick: CFAbsoluteTime = CFAbsoluteTimeGetCurrent()
    private var waveformPublishGate = WaveformPublishGate()

    var latchEngaged: Bool { recordingGate.latchEngaged }

    var shouldRecord: Bool { recordingGate.shouldRecord }

    func setReduceMotion(_ on: Bool) {
        reduceMotion = on
    }

    var onText: ((String) -> Void)?
    var onWaveform: (([Float]) -> Void)?
    var onPermissionBlocked: ((Bool) -> Void)?
    var onRequiresOnDeviceUnsupported: ((Bool) -> Void)?
    var onRecordingState: ((Bool) -> Void)?

    override init() {
        speechRecognizer = SFSpeechRecognizer(locale: Locale(identifier: "en_US"))
        super.init()
        configObserver = NotificationCenter.default.addObserver(
            forName: .AVAudioEngineConfigurationChange,
            object: audioEngine,
            queue: .main
        ) { [weak self] _ in
            Task { @MainActor in
                self?.handleRouteChange()
            }
        }
    }

    deinit {
        if let configObserver {
            NotificationCenter.default.removeObserver(configObserver)
        }
    }

    func toggleLatch(userPrefix: String) {
        let before = recordingGate.shouldRecord
        recordingGate.toggleLatch()
        let after = recordingGate.shouldRecord
        if after, !before {
            composer.resetForNewSession(keepingUserPrefix: userPrefix)
            silenceWatch.reset()
        }
        syncMicToGate()
    }

    func setFnHeld(_ down: Bool, userPrefix: String) {
        let before = recordingGate.shouldRecord
        recordingGate.setFnHeld(down)
        let after = recordingGate.shouldRecord
        if after, !before {
            composer.resetForNewSession(keepingUserPrefix: userPrefix)
            silenceWatch.reset()
        }
        syncMicToGate()
    }

    private func syncMicToGate() {
        onRecordingState?(recordingGate.shouldRecord)
        if recordingGate.shouldRecord {
            Task { await startRecordingIfAllowed() }
        } else {
            stopRecording()
        }
    }

    func preparePermissions() {
        requestMic()
        requestSpeech()
    }

    private func requestMic() {
        switch AVCaptureDevice.authorizationStatus(for: .audio) {
        case .authorized:
            onPermissionBlocked?(false)
        case .denied, .restricted:
            onPermissionBlocked?(true)
        case .notDetermined:
            AVCaptureDevice.requestAccess(for: .audio) { [weak self] ok in
                Task { @MainActor in
                    self?.onPermissionBlocked?(!ok)
                }
            }
        @unknown default:
            onPermissionBlocked?(true)
        }
    }

    // macOS 26 (Tahoe) changed TCC: calling requestAuthorization on an ad-hoc-signed
    // process crashes it with SIGABRT regardless of the bundle Info.plist. Developer-
    // signed apps (non-empty TeamIdentifier) get a permission dialog as expected.
    // Ad-hoc builds (dev mode, no team ID) must skip the call and surface the blocked
    // state through the normal UI instead.
    private static let canRequestSpeechAuth: Bool = {
        var staticCode: SecStaticCode?
        let cfURL = Bundle.main.bundleURL as CFURL
        guard SecStaticCodeCreateWithPath(cfURL, SecCSFlags(), &staticCode) == errSecSuccess,
              let staticCode else { return false }
        var cfInfo: CFDictionary?
        guard SecCodeCopySigningInformation(staticCode, SecCSFlags(), &cfInfo) == errSecSuccess,
              let info = cfInfo as? [String: Any] else { return false }
        let teamID = info[kSecCodeInfoTeamIdentifier as String] as? String ?? ""
        return !teamID.isEmpty
    }()

    private func requestSpeech() {
        guard Self.canRequestSpeechAuth else {
            onPermissionBlocked?(true)
            return
        }
        SFSpeechRecognizer.requestAuthorization { [weak self] status in
            Task { @MainActor in
                let ok = status == .authorized
                let micBad = self?.micDeniedEffective() ?? false
                self?.onPermissionBlocked?(!ok || micBad)
            }
        }
    }

    private func micDeniedEffective() -> Bool {
        switch AVCaptureDevice.authorizationStatus(for: .audio) {
        case .denied, .restricted: return true
        default: return false
        }
    }

    private func startRecordingIfAllowed() async {
        if isEngineRunning, recognitionRequest != nil, recognitionTask != nil {
            return
        }
        preparePermissions()
        if micDeniedEffective() {
            recordingGate = DictationRecordingGate()
            onRecordingState?(false)
            onPermissionBlocked?(true)
            return
        }
        guard let recognizer = speechRecognizer, recognizer.isAvailable else {
            recordingGate = DictationRecordingGate()
            onRecordingState?(false)
            onPermissionBlocked?(true)
            return
        }
        let onDeviceOK = recognizer.supportsOnDeviceRecognition
        onRequiresOnDeviceUnsupported?(!onDeviceOK)
        guard onDeviceOK else {
            recordingGate = DictationRecordingGate()
            onRecordingState?(false)
            return
        }

        stopRecording()
        lastSilenceTick = CFAbsoluteTimeGetCurrent()

        let inputNode = audioEngine.inputNode
        let inputFormat = inputNode.outputFormat(forBus: 0)

        guard let want = AVAudioFormat(
            commonFormat: .pcmFormatFloat32,
            sampleRate: 16000,
            channels: 1,
            interleaved: false
        ) else { return }

        targetFormat = want
        if abs(inputFormat.sampleRate - want.sampleRate) < 1, inputFormat.channelCount == want.channelCount {
            converter = nil
        } else {
            converter = AVAudioConverter(from: inputFormat, to: want)
            if converter == nil { return }
        }

        recognitionRequest = SFSpeechAudioBufferRecognitionRequest()
        guard let recognitionRequest else { return }
        recognitionRequest.shouldReportPartialResults = true
        recognitionRequest.requiresOnDeviceRecognition = true

        recognitionTask = recognizer.recognitionTask(with: recognitionRequest) { [weak self] result, error in
            Task { @MainActor in
                guard let self else { return }
                if let result {
                    let text = result.bestTranscription.formattedString
                    if result.isFinal {
                        self.composer.applyFinal(text)
                    } else {
                        self.composer.applyPartial(text)
                    }
                    self.onText?(self.composer.displayText)
                }
                if error != nil, !self.recordingGate.shouldRecord {
                    self.teardownRecognitionOnly()
                }
            }
        }

        inputNode.removeTap(onBus: 0)
        inputNode.installTap(onBus: 0, bufferSize: 2048, format: inputFormat) { [weak self] buffer, _ in
            guard let self, let dup = Self.duplicatePCM(buffer) else { return }
            let wall = CFAbsoluteTimeGetCurrent()
            Task { @MainActor in
                self.handleAudioBuffer(dup, wallTime: wall)
            }
        }

        audioEngine.prepare()
        do {
            try audioEngine.start()
            isEngineRunning = true
        } catch {
            isEngineRunning = false
            teardownRecognitionOnly()
        }

        pushWaveformIdle()
    }

    private func handleAudioBuffer(_ buffer: AVAudioPCMBuffer, wallTime: CFAbsoluteTime) {
        let toAppend = convertIfNeeded(buffer) ?? buffer
        recognitionRequest?.append(toAppend)

        let rms = Self.rmsFloatChannel(of: toAppend)
        let dt = max(0, wallTime - lastSilenceTick)
        lastSilenceTick = wallTime
        let levels = waveform.push(rms: rms, reduceMotion: reduceMotion)
        if waveformPublishGate.shouldPublish(levels, now: wallTime) {
            onWaveform?(levels)
        }
        if recordingGate.shouldRecord, silenceWatch.feed(rms: rms, deltaTime: dt) {
            stopAfterSilence()
        }
    }

    private func convertIfNeeded(_ buffer: AVAudioPCMBuffer) -> AVAudioPCMBuffer? {
        guard let converter, let want = targetFormat else { return nil }
        let ratio = want.sampleRate / buffer.format.sampleRate
        let outFrames = AVAudioFrameCount((Double(buffer.frameLength) * ratio).rounded(.up)) + 32
        guard let out = AVAudioPCMBuffer(pcmFormat: want, frameCapacity: outFrames) else { return nil }
        let box = ConsumeOnce()
        let inputBlock: AVAudioConverterInputBlock = { _, outStatus in
            if box.done {
                outStatus.pointee = .noDataNow
                return nil
            }
            box.done = true
            outStatus.pointee = .haveData
            return buffer
        }
        var err: NSError?
        converter.convert(to: out, error: &err, withInputFrom: inputBlock)
        if err != nil { return nil }
        return out
    }

    nonisolated private static func duplicatePCM(_ buffer: AVAudioPCMBuffer) -> AVAudioPCMBuffer? {
        let frames = buffer.frameLength
        guard frames > 0,
              let dup = AVAudioPCMBuffer(pcmFormat: buffer.format, frameCapacity: frames)
        else { return nil }
        dup.frameLength = frames
        let ch = Int(buffer.format.channelCount)
        guard let src = buffer.floatChannelData, let dst = dup.floatChannelData else { return nil }
        let n = Int(frames)
        for c in 0..<ch {
            dst[c].update(from: src[c], count: n)
        }
        return dup
    }

    private static func rmsFloatChannel(of buffer: AVAudioPCMBuffer) -> Float {
        guard let ch = buffer.floatChannelData else { return 0 }
        let n = Int(buffer.frameLength)
        guard n > 0 else { return 0 }
        let p = ch[0]
        var sum: Float = 0
        for i in 0..<n { sum += p[i] * p[i] }
        return sqrt(sum / Float(n))
    }

    private func stopAfterSilence() {
        silenceWatch.reset()
        if recordingGate.latchEngaged {
            recordingGate.toggleLatch()
        }
        recordingGate.setFnHeld(false)
        stopRecording()
    }

    func stopRecording() {
        // First access to `audioEngine.inputNode` initializes Core Audio HAL,
        // which triggers TCC microphone + speech-recognition checks. If nothing
        // was ever started (cold `start()` → `stopPolling()` teardown path), do
        // not touch the engine — that path must not request privacy-sensitive
        // resources before the user has engaged dictation.
        guard isEngineRunning || recognitionRequest != nil || recognitionTask != nil else {
            waveform = WaveformNineDotRing()
            silenceWatch.reset()
            waveformPublishGate.reset()
            pushWaveformIdle()
            onRecordingState?(recordingGate.shouldRecord)
            return
        }
        audioEngine.inputNode.removeTap(onBus: 0)
        if audioEngine.isRunning {
            audioEngine.stop()
        }
        isEngineRunning = false
        recognitionRequest?.endAudio()
        recognitionRequest = nil
        recognitionTask?.cancel()
        recognitionTask = nil
        waveform = WaveformNineDotRing()
        silenceWatch.reset()
        waveformPublishGate.reset()
        pushWaveformIdle()
        onRecordingState?(recordingGate.shouldRecord)
    }

    private func teardownRecognitionOnly() {
        recognitionRequest?.endAudio()
        recognitionRequest = nil
        recognitionTask?.cancel()
        recognitionTask = nil
    }

    private func handleRouteChange() {
        guard recordingGate.shouldRecord, isEngineRunning else { return }
        stopRecording()
        Task { await startRecordingIfAllowed() }
    }

    private func pushWaveformIdle() {
        let levels = waveform.push(rms: 0, reduceMotion: reduceMotion)
        if waveformPublishGate.shouldPublish(levels, now: CFAbsoluteTimeGetCurrent(), force: true) {
            onWaveform?(levels)
        }
    }

    func stopAllForHostTeardown() {
        recordingGate = DictationRecordingGate()
        stopRecording()
    }

    func openSystemPrivacySettings() {
        if let url = URL(string: "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone") {
            NSWorkspace.shared.open(url)
        }
    }
}
