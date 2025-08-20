#include <windows.h>
#include <iostream>
#include <iomanip>

class FileHeaderReader {
private:
    std::string filePath;

public:
    FileHeaderReader(const std::string& path) : filePath(path) {}

    bool readFirstTwoBytes() {
        // Dosyayý okuma modunda aç
        HANDLE hFile = CreateFileA(
            filePath.c_str(),           // Dosya yolu
            GENERIC_READ,               // Okuma eriþimi
            FILE_SHARE_READ,            // Paylaþým modu
            NULL,                       // Güvenlik atributu
            OPEN_EXISTING,              // Mevcut dosyayý aç
            FILE_ATTRIBUTE_NORMAL,      // Normal dosya atributu
            NULL                        // Template dosya
        );

        if (hFile == INVALID_HANDLE_VALUE) {
            std::cerr << "HATA: Dosya acilamadi - " << filePath << std::endl;
            std::cerr << "Hata Kodu: " << GetLastError() << std::endl;
            return false;
        }

        // Ýlk iki baytý okumak için buffer
        BYTE buffer[2];
        DWORD bytesRead = 0;

        // Dosyadan iki bayt oku
        BOOL result = ReadFile(
            hFile,          // Dosya handle
            buffer,         // Buffer
            2,              // Okunacak bayt sayýsý
            &bytesRead,     // Okunan bayt sayýsý
            NULL            // Overlapped yapýsý (kullanýlmýyor)
        );

        // Dosyayý kapat
        CloseHandle(hFile);

        if (!result) {
            std::cerr << "HATA: Dosya okunamadi!" << std::endl;
            std::cerr << "Hata Kodu: " << GetLastError() << std::endl;
            return false;
        }

        if (bytesRead == 0) {
            std::cout << "UYARI: Dosya bos!" << std::endl;
            return true;
        }

        // Sonuçlarý göster
        displayResults(buffer, bytesRead);
        return true;
    }

private:
    void displayResults(const BYTE* buffer, DWORD bytesRead) {
        std::cout << "\n=== DOSYA BASLIGI OKUYUCU ===" << std::endl;
        std::cout << "Dosya: " << filePath << std::endl;
        std::cout << "Okunan bayt sayisi: " << bytesRead << std::endl;
        std::cout << "\nDosyanin ilk iki bayti: ";

        // ASCII karakterler olarak göster (eðer yazdýrýlabilirse)
        for (DWORD i = 0; i < bytesRead; i++) {
            if (buffer[i] >= 32 && buffer[i] <= 126) { // Yazdýrýlabilir ASCII
                std::cout << static_cast<char>(buffer[i]);
            }
            else {
                std::cout << ".";
            }
        }

        std::cout << std::endl;

        // Hex formatýnda göster
        std::cout << "Hex formatinda: ";
        for (DWORD i = 0; i < bytesRead; i++) {
            std::cout << "0x" << std::hex << std::uppercase
                << std::setfill('0') << std::setw(2)
                << static_cast<int>(buffer[i]);
            if (i < bytesRead - 1) std::cout << " ";
        }
        std::cout << std::dec << std::endl; // Decimal'e geri dön

        // Decimal formatýnda göster
        std::cout << "Decimal formatinda: ";
        for (DWORD i = 0; i < bytesRead; i++) {
            std::cout << static_cast<int>(buffer[i]);
            if (i < bytesRead - 1) std::cout << " ";
        }
        std::cout << std::endl;

        // Dosya türü tahmini
        predictFileType(buffer, bytesRead);
    }

    void predictFileType(const BYTE* buffer, DWORD bytesRead) {
        if (bytesRead < 2) return;

        std::cout << "\nDosya turu tahmini: ";

        // Yaygýn dosya baþlýklarý
        if (buffer[0] == 0x4D && buffer[1] == 0x5A) {
            std::cout << "PE Executable (.exe, .dll)";
        }
        else if (buffer[0] == 0xFF && buffer[1] == 0xD8) {
            std::cout << "JPEG Resim";
        }
        else if (buffer[0] == 0x89 && buffer[1] == 0x50) {
            std::cout << "PNG Resim";
        }
        else if (buffer[0] == 0x50 && buffer[1] == 0x4B) {
            std::cout << "ZIP Arsivi (veya Office belgesi)";
        }
        else if (buffer[0] == 0x1F && buffer[1] == 0x8B) {
            std::cout << "GZIP Arsivi";
        }
        else if (buffer[0] == 0x42 && buffer[1] == 0x4D) {
            std::cout << "BMP Resim";
        }
        else if (buffer[0] == 0x47 && buffer[1] == 0x49) {
            std::cout << "GIF Resim";
        }
        else if (buffer[0] == 0x25 && buffer[1] == 0x50) {
            std::cout << "PDF Belgesi";
        }
        else if (buffer[0] == 0x52 && buffer[1] == 0x61) {
            std::cout << "RAR Arsivi";
        }
        else {
            std::cout << "Bilinmeyen format";
        }
        std::cout << std::endl;
    }
};

void showUsage(const char* programName) {
    std::cout << "\n=== DOSYA BASLIGI OKUYUCU ===" << std::endl;
    std::cout << "Kullanim: " << programName << " <dosya_yolu>" << std::endl;
    std::cout << "\nOrnek:" << std::endl;
    std::cout << "  " << programName << " C:\\Windows\\notepad.exe" << std::endl;
    std::cout << "  " << programName << " \"C:\\Program Files\\dosya.txt\"" << std::endl;
    std::cout << "\nAciklama:" << std::endl;
    std::cout << "  Belirtilen dosyanin ilk iki baytini okur ve farkli" << std::endl;
    std::cout << "  formatlarda (ASCII, Hex, Decimal) gosterir." << std::endl;
    std::cout << "  Ayrica dosya turu tahmini yapar." << std::endl;
}

int main(int argc, char* argv[]) {
    // Türkçe karakterler için konsol kodlamasýný ayarla
    SetConsoleOutputCP(CP_UTF8);
    SetConsoleCP(CP_UTF8);

    // Komut satýrý argümanlarý kontrolü
    if (argc != 2) {
        showUsage(argv[0]);
        std::cout << "\nHATA: Lutfen bir dosya yolu belirtin!" << std::endl;
        return 1;
    }

    std::string filePath = argv[1];

    // Dosya yolunun boþ olup olmadýðýný kontrol et
    if (filePath.empty()) {
        std::cerr << "HATA: Gecersiz dosya yolu!" << std::endl;
        showUsage(argv[0]);
        return 1;
    }

    // Dosya okuyucu nesnesini oluþtur ve çalýþtýr
    FileHeaderReader reader(filePath);

    if (!reader.readFirstTwoBytes()) {
        return 1; // Hata durumu
    }

    std::cout << "\nIslem tamamlandi." << std::endl;
    return 0;
}