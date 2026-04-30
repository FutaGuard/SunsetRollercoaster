from typing import Any

import ddddocr
from bs4 import BeautifulSoup
from tomlkit import table

from sunsetRollercoaster import crawler
from sunsetRollercoaster.crawler._crawler import Crawler


class CarCheck(Crawler):
    def __init__(self, proxy: Crawler.Proxy = Crawler.Proxy.JP):
        super().__init__(proxy)

    async def query(self, url):
        return await super().query(url)

    async def check_car(self, queryType: str,companyNo: str='',plateNo: str='',idNo: str='',birthday: str='') -> Any:
        '''
        check_car 查詢車輛定檢日
        :param queryType: 1是個人 2是公司
        :param companyNo: 身分證或是統編
        :param plateNo: 車牌號碼
        '''
        #
        url ='https://www.mvdis.gov.tw/m3-emv-car/car/checkQuery'
        resp = await self.client.get(url)

        captcha_img = await self.client.get('https://www.mvdis.gov.tw//m3-emv-car/captchaImg.jpg')
        ocr = ddddocr.DdddOcr(show_ad=False)
        captcha_code = ocr.classification(captcha_img.content)

        data = {
            'method': 'queryCheck',
            'queryType': queryType,
            'queryMode': '1',
            'companyNo': companyNo,
            'plateNo': plateNo,
            'idNo': idNo,
            'birthday': birthday,
            'perPlateNo': '',
            'carType': '1',
            'validateStr': captcha_code,
        }
        resp = await self.client.post('https://www.mvdis.gov.tw/m3-emv-car/car/checkQuery', data=data)
        soup = BeautifulSoup(resp.text, 'html.parser')
        table = soup.find('table', id='info')
        # rows = table.find('tbody').find_all('tr')
        results = []
        for row in table.find("tbody").find_all("tr"):
            cells = row.find_all("td")
            results.append({
                "車種": cells[0].get_text(strip=True),
                "車號": cells[1].get_text(strip=True),
                "定檢日": cells[2].get_text(strip=True),
            })

        return results
