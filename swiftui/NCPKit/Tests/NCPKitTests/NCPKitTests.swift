import XCTest
@testable import NCPKit

final class NCPKitTests: XCTestCase {
    func testVersionStringNotEmpty() {
        XCTAssertFalse(NCPClient.version().isEmpty)
    }
}
