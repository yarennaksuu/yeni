#include <iostream>
#include <fstream>
#include <string>
#include <vector>
#include <algorithm>
#include <iomanip>
#include <windows.h>

class ContentSearcher {
private:
    std::string filePath;
    std::string searchString;

    // Dosya boyutunu al
    size_t getFileSize(const std::string& fileName) {
        std::ifstream file(fileName, std::ios::binary | std::ios::ate);
        if (!file.is_open()) {
            return 0;
        }
        return static_cast<size_t>(file.tellg());
    }

    // Hexadecimal formatta adres yazd�r
    void printHexAddress(size_t address) {
        std::cout << std::uppercase << std::hex << address << std::dec;
    }

    // String'i case-insensitive kar��la�t�rma i�in k���k harfe �evir
    std::string toLower(const std::string& str) {
        std::string result = str;
        std::transform(result.begin(), result.end(), result.begin(), ::tolower);
        return result;
    }

public:
    ContentSearcher(const std::string& path, const std::string& search)
        : filePath(path), searchString(search) {
    }

    // Ana arama fonksiyonu
    bool searchInFile() {
        std::ifstream file(filePath, std::ios::binary);
        if (!file.is_open()) {
            std::cerr << "Hata: '" << filePath << "' dosyasi acilamadi!" << std::endl;
            return false;
        }

        // Dosya boyutunu kontrol et
        size_t fileSize = getFileSize(filePath);
        if (fileSize == 0) {
            std::cerr << "Hata: Dosya bos veya okunamiyor!" << std::endl;
            return false;
        }

        std::cout << "Dosya: " << filePath << std::endl;
        std::cout << "Aranan: " << searchString << std::endl;
        std::cout << "Dosya boyutu: " << fileSize << " byte" << std::endl;
        std::cout << "Arama baslatiliyor..." << std::endl << std::endl;

        // Buffer boyutu - performans i�in optimize edilmi�
        const size_t BUFFER_SIZE = 8192;
        std::vector<char> buffer(BUFFER_SIZE + searchString.length() - 1);

        size_t totalBytesRead = 0;
        size_t foundCount = 0;
        bool found = false;

        // Case-insensitive arama i�in k���k harfe �evir
        std::string searchLower = toLower(searchString);

        while (!file.eof()) {
            // Buffer'� temizle
            std::fill(buffer.begin(), buffer.end(), 0);

            // Veriyi oku
            file.read(buffer.data(), BUFFER_SIZE);
            size_t bytesRead = static_cast<size_t>(file.gcount());

            if (bytesRead == 0) break;

            // String'i k���k harfe �evir
            std::string bufferStr(buffer.data(), bytesRead + searchString.length() - 1);
            std::string bufferLower = toLower(bufferStr);

            // Arama i�lemi
            size_t pos = 0;
            while ((pos = bufferLower.find(searchLower, pos)) != std::string::npos) {
                size_t actualAddress = totalBytesRead + pos;

                std::cout << "BULUNDU: '" << searchString << "' ifadesi 0x";
                printHexAddress(actualAddress);
                std::cout << " (" << actualAddress << ") adresinde bulundu." << std::endl;

                found = true;
                foundCount++;
                pos++;
            }

            totalBytesRead += bytesRead;

            // �lerleme g�stergesi (b�y�k dosyalar i�in)
            if (fileSize > 1024 * 1024) { // 1MB'dan b�y�kse
                int progress = static_cast<int>((totalBytesRead * 100) / fileSize);
                if (progress % 10 == 0) {
                    std::cout << "\rIlerleme: %" << progress << std::flush;
                }
            }

            // Overlap i�in geri git
            if (!file.eof() && bytesRead >= searchString.length()) {
                file.seekg(-(static_cast<std::streamoff>(searchString.length() - 1)), std::ios::cur);
                totalBytesRead -= (searchString.length() - 1);
            }
        }

        file.close();

        if (fileSize > 1024 * 1024) {
            std::cout << "\r"; // �lerleme sat�r�n� temizle
        }

        std::cout << std::endl << "Arama tamamlandi." << std::endl;

        if (found) {
            std::cout << "Toplam " << foundCount << " adet '" << searchString << "' bulundu." << std::endl;
        }
        else {
            std::cout << "'" << searchString << "' bulunamadi." << std::endl;
        }

        return found;
    }

    // Dosya bilgilerini g�ster
    void showFileInfo() {
        WIN32_FIND_DATAA findData;
        HANDLE hFind = FindFirstFileA(filePath.c_str(), &findData);

        if (hFind != INVALID_HANDLE_VALUE) {
            LARGE_INTEGER fileSize;
            fileSize.LowPart = findData.nFileSizeLow;
            fileSize.HighPart = findData.nFileSizeHigh;

            std::cout << "\n=== DOSYA BILGILERI ===" << std::endl;
            std::cout << "Dosya adi: " << findData.cFileName << std::endl;
            std::cout << "Boyut: " << fileSize.QuadPart << " byte" << std::endl;
            std::cout << "Ozellikler: ";

            if (findData.dwFileAttributes & FILE_ATTRIBUTE_READONLY)
                std::cout << "Salt-Okunur ";
            if (findData.dwFileAttributes & FILE_ATTRIBUTE_HIDDEN)
                std::cout << "Gizli ";
            if (findData.dwFileAttributes & FILE_ATTRIBUTE_SYSTEM)
                std::cout << "Sistem ";
            if (findData.dwFileAttributes & FILE_ATTRIBUTE_ARCHIVE)
                std::cout << "Arsiv ";

            std::cout << std::endl;
            FindClose(hFind);
        }
    }
};

void showUsage(const char* programName) {
    std::cout << "=== SABIT ICERIK ARAYICISI ===" << std::endl;
    std::cout << "Windows 10/11 Uyumlu - Dis Kutuphanesi Gerektirmez" << std::endl;
    std::cout << std::endl;
    std::cout << "Kullanim:" << std::endl;
    std::cout << "  " << programName << " <dosya_yolu> <aranan_string>" << std::endl;
    std::cout << std::endl;
    std::cout << "Ornekler:" << std::endl;
    std::cout << "  " << programName << " C:\\test\\dosya.exe MALWARE" << std::endl;
    std::cout << "  " << programName << " \"C:\\Program Files\\test.dll\" virus" << std::endl;
    std::cout << "  " << programName << " ./local_file.bin suspicious" << std::endl;
    std::cout << std::endl;
    std::cout << "Not: Arama case-insensitive (buyuk/kucuk harf duyarsiz) yapilir." << std::endl;
    std::cout << "     Bulunan her konum icin hem hex hem decimal adres gosterilir." << std::endl;
}

int main(int argc, char* argv[]) {
    // Windows konsol ayarlar� - T�rk�e karakter deste�i
    SetConsoleOutputCP(CP_UTF8);
    SetConsoleCP(CP_UTF8);

    std::cout << "=== SABIT ICERIK ARAYICISI v1.0 ===" << std::endl;
    std::cout << "Windows Uyumlu Dosya Icerik Arama Araci" << std::endl;
    std::cout << "========================================" << std::endl << std::endl;

    // Parametre kontrol�
    if (argc < 3) {
        showUsage(argv[0]);
        return 1;
    }

    std::string filePath = argv[1];
    std::string searchString = argv[2];

    // Dosya varl���n� kontrol et
    std::ifstream testFile(filePath);
    if (!testFile.good()) {
        std::cerr << "Hata: '" << filePath << "' dosyasi bulunamadi veya erisim hatasi!" << std::endl;
        return 2;
    }
    testFile.close();

    // Arama stringinin ge�erlili�ini kontrol et
    if (searchString.empty()) {
        std::cerr << "Hata: Aranan string bos olamaz!" << std::endl;
        return 3;
    }

    // ContentSearcher nesnesini olu�tur ve aramay� ba�lat
    ContentSearcher searcher(filePath, searchString);

    // Dosya bilgilerini g�ster
    searcher.showFileInfo();

    std::cout << std::endl;

    // Arama i�lemini ger�ekle�tir
    bool result = searcher.searchInFile();

    std::cout << std::endl << "========================================" << std::endl;
    std::cout << "Islem " << (result ? "BASARILI" : "TAMAMLANDI") << " - Program sonlaniyor." << std::endl;

    return result ? 0 : 4;
}

// Derleme talimatlar�:
// g++ -std=c++11 -O2 -o content_searcher.exe content_searcher.cpp
// 
// Veya Microsoft Visual Studio ile:
// cl /EHsc /O2 content_searcher.cpp
//
// Kullanim ornekleri:
// content_searcher.exe C:\test\malware.exe MALWARE
// content_searcher.exe "C:\Program Files\app.dll" suspicious
// content_searcher.exe ./test.bin virus